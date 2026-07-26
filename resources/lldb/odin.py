# Original author: laytan (https://gist.github.com/laytan/a94c323a84cef7bcfbdf6d21987fd5a9)
# Modifications by: harold-b (https://gist.github.com/harold-b/ef16a5c3ebcceccfc2bc7a5c5dd0058d)
# Modifications by: adneufeld
# Modifications by: rxptr

import logging
import math
import struct

import lldb

log = logging.getLogger(__name__)


def is_slice_type(t, internal_dict):
    return (
        t.name.startswith("[]") or t.name.startswith("[dynamic]")
    ) and not t.name.endswith("]")


def slice_summary(value, internal_dict):
    value = value.GetNonSyntheticValue()
    length = value.GetChildMemberWithName("len").unsigned
    data = value.GetChildMemberWithName("data")

    pointee = data.deref
    type_name = pointee.type.GetDisplayTypeName()

    return f"[{length}]{type_name}"


class SliceChildProvider:
    CHUNK_COUNT = 2000
    MAX_ELEMENTS = 1 << 20

    def __init__(self, val, dict):
        self.val = val
        self.update()

    def update(self):
        val = self.val

        self.len = val.GetChildMemberWithName("len").GetValueAsSigned()
        self.data_val = val.GetChildMemberWithName("data")
        if (
            self.len < 0
            or not self.data_val.type.is_pointer
            or self.data_val.GetValueAsUnsigned() == 0
        ):
            self.len = 0
        self.len = min(self.len, SliceChildProvider.MAX_ELEMENTS)

        is_chunked = self.len > SliceChildProvider.CHUNK_COUNT
        self.chunked_len = (
            0
            if not is_chunked
            else math.ceil(self.len / SliceChildProvider.CHUNK_COUNT)
        )

        return False

    def num_children(self):
        return self.chunked_len if self.chunked_len > 0 else self.len

    def get_child_at_index(self, index):
        if index < 0 or index >= self.num_children():
            return None

        first = self.data_val.deref
        if not first.IsValid():
            return None

        if self.chunked_len > 0:
            chunk_size = SliceChildProvider.CHUNK_COUNT

            array_len = min(chunk_size, self.len - index * chunk_size)
            arr_type = first.type.GetArrayType(array_len)
            offset = index * first.size * chunk_size

            range_start = index * chunk_size

            return self.data_val.CreateChildAtOffset(
                f"[{range_start}..<{range_start + array_len}]", offset, arr_type
            )

        offset = index * first.size
        return self.data_val.CreateChildAtOffset(f"[{index}]", offset, first.type)


MAX_DISPLAY_LENGTH = 4096


def is_string_type(t, internal_dict):
    return t.name == "string"


def string_summary(value, internal_dict):
    pointer = value.GetChildMemberWithName("data").GetValueAsUnsigned(0)
    length = value.GetChildMemberWithName("len").GetValueAsSigned(0)
    if length <= 0:
        return '""'
    if pointer == 0:
        return "nil"
    error = lldb.SBError()
    read_length = min(length, MAX_DISPLAY_LENGTH)
    string_data = value.process.ReadMemory(pointer, read_length, error)
    if not error.success:
        return "<unreadable>"
    text = string_data.decode("utf-8", errors="replace")
    suffix = "…" if length > read_length else ""
    return '"{}{}"'.format(text, suffix)




def is_map_type(t, internal_dict):
    return t.name.startswith("map[")


def is_bit_set_type(t, internal_dict):
    return t.name.startswith("bit_set[")


def bit_set_summary(value, internal_dict):
    if value.IsSynthetic():
        value = value.GetNonSyntheticValue()

    is_rune_range = "rune(" in value.type.name
    members = []
    for child in value.children:
        name = child.name
        if not name or child.type.name != "bool" or child.unsigned == 0:
            continue
        if name.isdigit():
            codepoint = int(name)
            members.append("'{}'".format(chr(codepoint)) if is_rune_range else name)
        else:
            members.append(".{}".format(name))
    return "{{{}}}".format(", ".join(members))


MAX_MAP_SCAN = 1 << 20
TOMBSTONE_MASK = 1 << 63


def map_summary(value, internal_dict):
    value = value.GetNonSyntheticValue()
    length = value.GetChildMemberWithName("len").GetValueAsSigned(0)
    raw_data = value.GetChildMemberWithName("data").GetValueAsUnsigned(0)
    cap_log2 = raw_data & 63
    cap = 1 << cap_log2 if cap_log2 > 0 else 0
    return "len = {}, cap = {}".format(length, cap)


class MapChildProvider:
    def __init__(self, val, dict):
        self.val = val
        self.update()

    def update(self):
        self.entries = []

        val = self.val
        length = val.GetChildMemberWithName("len").GetValueAsSigned()
        data = val.GetChildMemberWithName("data")
        if length <= 0 or not data.IsValid():
            return False

        self.tkey = data.GetChildMemberWithName("key").type
        self.tval = data.GetChildMemberWithName("value").type
        hash_field = data.GetChildMemberWithName("hash")
        key_cell = data.GetChildMemberWithName("key_cell")
        value_cell = data.GetChildMemberWithName("value_cell")

        raw_data = data.GetValueAsUnsigned()
        key_ptr = raw_data & ~63
        cap_log2 = raw_data & 63
        cap = 1 << cap_log2 if cap_log2 > 0 else 0
        if key_ptr == 0 or cap == 0 or hash_field.size != 8:
            return False

        key_cell_info = self.cell_info(self.tkey, key_cell)
        value_cell_info = self.cell_info(self.tval, value_cell)

        value_ptr = self.cell_index(key_ptr, key_cell_info, cap)
        hash_ptr = self.cell_index(value_ptr, value_cell_info, cap)

        scan_cap = min(cap, MAX_MAP_SCAN)
        error = lldb.SBError()
        hash_bytes = val.process.ReadMemory(hash_ptr, scan_cap * 8, error)
        if not error.success:
            return False

        for i, hash_val in enumerate(struct.unpack(f"<{scan_cap}Q", hash_bytes)):
            if hash_val == 0 or (hash_val & TOMBSTONE_MASK) != 0:
                continue
            self.entries.append(
                (
                    self.cell_index(key_ptr, key_cell_info, i),
                    self.cell_index(value_ptr, value_cell_info, i),
                )
            )
            if len(self.entries) >= length:
                break

        return False

    def num_children(self):
        return len(self.entries)

    def get_child_at_index(self, index):
        if index < 0 or index >= len(self.entries):
            return None

        key_addr, value_addr = self.entries[index]
        key = self.val.CreateValueFromAddress("key", key_addr, self.tkey)
        label = key.GetSummary() or key.GetValue() or str(index)
        return self.val.CreateValueFromAddress(
            f"[{label}]", value_addr, self.tval
        )

    def cell_info(self, typev, cell_type):
        elements_per_cell = 0

        if typev.size != cell_type.size:
            array_type = cell_type.children[0].type
            if array_type.size > 0 and typev.size > 0:
                elements_per_cell = array_type.size / typev.size

        if elements_per_cell == 0:
            elements_per_cell = 1

        return CellInfo(typev.size, cell_type.size, elements_per_cell)

    def cell_index(self, base, info, index):
        cell_index = 0
        data_index = 0
        if info.elements_per_cell == 1:
            return base + (index * info.size_of_cell)
        elif info.elements_per_cell == 2:
            cell_index = index >> 1
            data_index = index & 1
        elif info.elements_per_cell == 4:
            cell_index = index >> 2
            data_index = index & 3
        elif info.elements_per_cell == 8:
            cell_index = index >> 3
            data_index = index & 7
        elif info.elements_per_cell == 16:
            cell_index = index >> 4
            data_index = index & 15
        elif info.elements_per_cell == 32:
            cell_index = index >> 5
            data_index = index & 31
        else:
            cell_index = index / info.elements_per_cell
            data_index = index % info.elements_per_cell

        return (
            base + (cell_index * info.size_of_cell) + (data_index * info.size_of_type)
        )


class CellInfo:
    def __init__(self, size_of_type, size_of_cell, elements_per_cell):
        self.size_of_type = size_of_type
        self.size_of_cell = size_of_cell
        self.elements_per_cell = elements_per_cell


class UnionChildProvider:
    def __init__(self, val, dict):
        self.val = val

    def update(self):
        self.children = self.val.children
        self.variant_index = self.children[0].unsigned if self.children else 0
        return False

    def num_children(self):
        return max(0, len(self.children) - 1)

    def get_child_at_index(self, index):
        if index < 0 or index >= self.num_children():
            return None

        value = self.val
        variant = self.children[index + 1]
        name = variant.type.GetDisplayTypeName()
        selected = "*" if variant.name == f"v{self.variant_index}" else ""

        field_name = f"{selected}{variant.name}({name})"
        return value.CreateValueFromData(field_name, variant.data, variant.type)


def is_type_union(t, internal_dict):
    if t.type != lldb.eTypeClassUnion:
        return False

    tag = t.GetFieldAtIndex(0)
    return tag and tag.IsValid() and tag.name == "tag"


def union_summary(value, internal_dict):
    if value.IsSynthetic():
        value = value.GetNonSyntheticValue()

    tag = value.GetChildAtIndex(0)
    if not tag.IsValid() or tag.name != "tag":
        return ""

    variant = value.GetChildMemberWithName(f"v{tag.unsigned}")
    if not variant.IsValid():
        return "nil"

    return f"{variant}"


def is_maybe_type(t, internal_dict):
    return t.name.startswith("runtime::Maybe(") or t.name.startswith("Maybe(")


def maybe_summary(value, internal_dict):
    if value.IsSynthetic():
        value = value.GetNonSyntheticValue()

    tag = value.GetChildAtIndex(0)
    if not tag.IsValid() or tag.name != "tag" or tag.unsigned == 0:
        return "nil"

    variant = value.GetChildMemberWithName(f"v{tag.unsigned}")
    if not variant.IsValid():
        return "nil"

    return variant.GetSummary() or variant.GetValue() or f"{variant}"


def __lldb_init_module(debugger, unused):
    debugger.HandleCommand(
        "type summary add --recognizer-function --python-function odin.union_summary odin.is_type_union"
    )
    debugger.HandleCommand(
        "type synth add --recognizer-function --python-class odin.UnionChildProvider odin.is_type_union"
    )

    debugger.HandleCommand(
        "type summary add --recognizer-function --python-function odin.string_summary odin.is_string_type"
    )
    debugger.HandleCommand(
        "type synth add --recognizer-function --python-class odin.SliceChildProvider odin.is_slice_type"
    )
    debugger.HandleCommand(
        "type summary add --recognizer-function --python-function odin.slice_summary odin.is_slice_type"
    )

    debugger.HandleCommand(
        "type synth add --recognizer-function --python-class odin.MapChildProvider odin.is_map_type"
    )
    debugger.HandleCommand(
        "type summary add --recognizer-function --python-function odin.map_summary odin.is_map_type"
    )
    debugger.HandleCommand(
        "type summary add --recognizer-function --python-function odin.bit_set_summary odin.is_bit_set_type"
    )
    debugger.HandleCommand(
        "type summary add --recognizer-function --python-function odin.maybe_summary odin.is_maybe_type"
    )
