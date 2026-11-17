import { dataTypeSpan, resolvePoints } from "@/lib/definition";
import type { Definition, RegisterPoint } from "@/types/models";

export interface MappingRange {
  startAddress: number;
  quantity: number;
}

export function mappingPoints(definition: Definition): RegisterPoint[] {
  return definition.points.length ? definition.points : resolvePoints(definition);
}

export function pointEnd(point: RegisterPoint): number {
  return point.address + dataTypeSpan(point.dataType);
}

export function cloneMappingPoint(point: RegisterPoint): RegisterPoint {
  return {
    ...point,
    alerts: point.alerts.map((rule) => ({ ...rule })),
    simulator: point.simulator ? { ...point.simulator } : undefined,
  };
}

export function mappingRange(points: readonly RegisterPoint[]): MappingRange {
  if (!points.length) return { startAddress: 0, quantity: 0 };
  const startAddress = Math.min(...points.map((point) => point.address));
  const end = Math.max(...points.map(pointEnd));
  return { startAddress, quantity: end - startAddress };
}

export function mappingRangeValid(points: readonly RegisterPoint[]): boolean {
  const range = mappingRange(points);
  return range.quantity <= 0xffff && range.startAddress + range.quantity <= 0x10000;
}

export function overlappingPoint(
  points: readonly RegisterPoint[],
  address: number,
  span: number,
  ignoreAddress?: number,
): RegisterPoint | null {
  const end = address + span;
  for (const point of points) {
    if (point.address === ignoreAddress) continue;
    const otherEnd = pointEnd(point);
    if (address < otherEnd && point.address < end) return point;
  }
  return null;
}
