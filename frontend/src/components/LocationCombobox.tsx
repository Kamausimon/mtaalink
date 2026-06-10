"use client";

import { useEffect, useRef, useState } from "react";
import { Input } from "@/components/ui/input";
import { KENYA_LOCATIONS } from "@/lib/locations";
import { cn } from "@/lib/utils";
import { MapPin } from "lucide-react";

type LocationComboboxProps = {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
  id?: string;
};

export function LocationCombobox({ value, onChange, placeholder, className, id }: LocationComboboxProps) {
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const query = value.trim().toLowerCase();
  const suggestions = query
    ? KENYA_LOCATIONS.filter((loc) => loc.toLowerCase().includes(query)).slice(0, 8)
    : KENYA_LOCATIONS.slice(0, 8);

  return (
    <div ref={containerRef} className="relative">
      <Input
        id={id}
        value={value}
        onChange={(e) => {
          onChange(e.target.value);
          setOpen(true);
        }}
        onFocus={() => setOpen(true)}
        placeholder={placeholder}
        className={className}
        autoComplete="off"
      />
      {open && suggestions.length > 0 && (
        <ul className="absolute z-20 mt-1 w-full max-h-56 overflow-auto rounded-lg border border-border bg-popover text-popover-foreground shadow-md py-1">
          {suggestions.map((loc) => (
            <li key={loc}>
              <button
                type="button"
                onClick={() => {
                  onChange(loc);
                  setOpen(false);
                }}
                className={cn(
                  "flex w-full items-center gap-2 px-3 py-1.5 text-sm text-left hover:bg-accent hover:text-accent-foreground transition-colors",
                  loc === value && "bg-accent/50",
                )}
              >
                <MapPin className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                {loc}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
