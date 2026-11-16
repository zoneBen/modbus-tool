export interface ChartSeries {
  key: number;
  label: string;
  color: string;
}

export type ChartData = [number[], ...Array<Array<number | null>>];
