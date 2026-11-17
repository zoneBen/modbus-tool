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
    points: definition.points.map((point) => ({
      ...point,
      alerts: point.alerts.map((rule) => ({ ...rule })),
      simulator: point.simulator ? { ...point.simulator } : undefined,
    })),
  };
}

export interface ResolvedPoint {
  address: number;
  dataType: DataType;
  byteOrder: ByteOrder;
  meaning: string;
  unit: string;
  scale: number;
  offset: number;
  alerts: Definition["points"][number]["alerts"];
  simulator?: Definition["points"][number]["simulator"];
}

export function dataTypeSpan(dataType: DataType): number {
  if (dataType === "f32" || dataType === "i32" || dataType === "u32") return 2;
  if (dataType === "f64" || dataType === "i64" || dataType === "u64") return 4;
  return 1;
}

export function resolvePoints(definition: Definition): ResolvedPoint[] {
  if (definition.points?.length) return definition.points;
  const span = dataTypeSpan(definition.dataType);
  const isBit = definition.functionCode === 1 || definition.functionCode === 2;
  const points: ResolvedPoint[] = [];
  if (isBit) {
    for (let offset = 0; offset < definition.quantity; offset += 1) {
      const address = definition.startAddress + offset;
      points.push({
        address,
        dataType: "bool",
        byteOrder: "abcd",
        meaning: definition.aliases[address] ?? "",
        unit: "",
        scale: 1,
        offset: 0,
        alerts: [],
      });
    }
    return points;
  }
  for (let offset = 0; offset + span <= definition.quantity; offset += span) {
    const address = definition.startAddress + offset;
    points.push({
      address,
      dataType: definition.dataType,
      byteOrder: definition.byteOrder,
      meaning: definition.aliases[address] ?? "",
      unit: "",
      scale: 1,
      offset: 0,
      alerts: [],
    });
  }
  return points;
}
