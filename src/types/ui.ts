import type { DataType } from "./models";

export interface WriteTarget {
  address: number;
  alias: string;
  current: string;
  dataType: DataType;
}
