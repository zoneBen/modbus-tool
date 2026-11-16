import type { ByteOrder, DataType, Definition, DefinitionInput } from "@/types/models";

export const DATA_TYPE_OPTIONS: Array<{ label: string; value: DataType }> = [
  { label: "BOOL", value: "bool" },
  { label: "INT8", value: "i8" },
  { label: "UINT8", value: "u8" },
  { label: "INT16", value: "i16" },
  { label: "UINT16", value: "u16" },
  { label: "INT32", value: "i32" },
  { label: "UINT32", value: "u32" },
  { label: "INT64", value: "i64" },
  { label: "UINT64", value: "u64" },
  { label: "FLOAT32", value: "f32" },
  { label: "FLOAT64", value: "f64" },
  { label: "HEX", value: "hex" },
  { label: "BIN", value: "bin" },
  { label: "ASCII", value: "ascii" },
];

export const BYTE_ORDER_OPTIONS: Array<{ label: string; value: ByteOrder }> = [
  { label: "ABCD · 大端", value: "abcd" },
  { label: "DCBA · 小端", value: "dcba" },
  { label: "BADC · 字节交换", value: "badc" },
  { label: "CDAB · 字交换", value: "cdab" },
];

export function toDefinitionInput(definition: Definition): DefinitionInput {
  return {
    name: definition.name,
    functionCode: definition.functionCode,
    slaveId: definition.slaveId,
    startAddress: definition.startAddress,
    quantity: definition.quantity,
    dataType: definition.dataType,
    byteOrder: definition.byteOrder,
    intervalMs: definition.intervalMs,
    enabled: definition.enabled,
    aliases: { ...definition.aliases },
  };
}
