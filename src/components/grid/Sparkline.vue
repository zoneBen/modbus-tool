<script setup lang="ts">
import { computed, useId } from "vue";

const props = withDefaults(
  defineProps<{
    points: number[];
    width?: number;
    height?: number;
    tone?: string;
    fill?: boolean;
  }>(),
  {
    width: 96,
    height: 26,
    tone: "var(--signal)",
    fill: true,
  },
);

const uid = useId().replace(/[^a-zA-Z0-9_-]/g, "");
const gradientId = `spark-${uid}`;

const geometry = computed(() => {
  const points = props.points.filter((point) => Number.isFinite(point));
  if (points.length < 2) return null;
  const min = Math.min(...points);
  const max = Math.max(...points);
  const span = max - min || 1;
  const step = 100 / (points.length - 1);
  const coords = points.map((value, index) => {
    const x = index * step;
    const y = 26 - ((value - min) / span) * 22 - 2;
    return [x, y] as const;
  });
  const line = coords.map(([x, y]) => `${x.toFixed(2)},${y.toFixed(2)}`).join(" ");
  const area = `M0,28 L${coords.map(([x, y]) => `${x.toFixed(2)},${y.toFixed(2)}`).join(" L")} L100,28 Z`;
  const last = coords[coords.length - 1] ?? null;
  return { line, area, last };
});
</script>

<template>
  <svg
    viewBox="0 0 100 28"
    :width="width"
    :height="height"
    preserveAspectRatio="none"
    class="overflow-visible"
    aria-hidden="true"
  >
    <defs>
      <linearGradient :id="gradientId" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" :stop-color="tone" stop-opacity="0.14" />
        <stop offset="1" :stop-color="tone" stop-opacity="0" />
      </linearGradient>
    </defs>
    <template v-if="geometry">
      <path v-if="fill" :d="geometry.area" :fill="`url(#${gradientId})`" />
      <polyline
        :points="geometry.line"
        fill="none"
        :stroke="tone"
        stroke-width="1.25"
        stroke-linecap="round"
        stroke-linejoin="round"
        vector-effect="non-scaling-stroke"
      />
      <circle
        v-if="geometry.last"
        :cx="geometry.last[0]"
        :cy="geometry.last[1]"
        r="1.8"
        :fill="tone"
      />
    </template>
    <line
      v-else
      x1="0"
      y1="26"
      x2="100"
      y2="26"
      stroke="currentColor"
      stroke-opacity="0.18"
      stroke-dasharray="2 3"
    />
  </svg>
</template>
