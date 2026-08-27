"use client";
// beui.dev/components/blocks/availability-scheduler

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/motion/select";
import type { TimeOption } from "./types";

// Time field: the library Select, with the option list capped so the panel
// measures a small height and scrolls instead of unfolding all 48 options.
export function TimeSelect({
  value,
  onChange,
  open,
  onOpenChange,
  options,
}: {
  value: string;
  onChange: (v: string) => void;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  options: TimeOption[];
}) {
  return (
    <Select
      value={value}
      onValueChange={onChange}
      open={open}
      onOpenChange={onOpenChange}
      className="w-full"
    >
      {/* LOCALLY MODIFIED: this window's 28px control scale and its own border,
          not the block's page-sized defaults. beUI draws controls with
          `border-border` (`--line`); every card and row in this window is drawn
          with the softer `border-hairline`, so the block's default made the time
          fields the most strongly outlined thing on the tab. See the note at the
          top of `day-row.tsx`. */}
      <SelectTrigger className="h-7 rounded-lg border-hairline px-2 text-callout tabular-nums">
        <SelectValue className="whitespace-nowrap" />
      </SelectTrigger>
      <SelectContent className="border-hairline">
        <div className="max-h-56 overflow-y-auto overscroll-contain">
          {options.map((o) => (
            <SelectItem key={o.value} value={o.value} className="tabular-nums">
              {o.label}
            </SelectItem>
          ))}
        </div>
      </SelectContent>
    </Select>
  );
}
