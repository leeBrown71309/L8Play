import { useEffect, useId, useRef, useState } from "react";
import type { InputHTMLAttributes, ReactNode, TextareaHTMLAttributes } from "react";

import { DsIcon, type DsIconName } from "@/components/ds/ds-icon";
import { cn } from "@/lib/cn";

interface FieldShellProps {
  label?: string | undefined;
  optional?: string | undefined;
  hint?: ReactNode;
  error?: string | undefined;
  icon?: DsIconName | undefined;
  action?: ReactNode;
  className?: string | undefined;
  id: string;
  children: ReactNode;
}

function FieldShell({ label, optional, hint, error, icon, action, className, id, children }: FieldShellProps) {
  return (
    <div className={cn("l8-field", error && "is-error", className)}>
      {label ? (
        <label htmlFor={id} className="l8-field-label">
          {label}
          {optional ? <span className="l8-field-opt">{optional}</span> : null}
        </label>
      ) : null}
      <div className="l8-field-box">
        {icon ? <DsIcon name={icon} size={18} className="l8-field-icon" /> : null}
        {children}
        {action ?? null}
      </div>
      {error || hint ? (
        <div id={`${id}-hint`} className="l8-field-hint">
          {error ? <DsIcon name="alertCircle" size={14} /> : null}
          {error ?? hint}
        </div>
      ) : null}
    </div>
  );
}

interface TextFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "className"> {
  label?: string;
  optional?: string;
  hint?: ReactNode;
  error?: string | undefined;
  icon?: DsIconName;
  action?: ReactNode;
  mono?: boolean;
  className?: string;
}

/** Single-line text field with label, hint and error. */
export function TextField({
  label,
  optional,
  hint,
  error,
  icon,
  action,
  mono,
  className,
  id,
  ...input
}: TextFieldProps) {
  const generated = useId();
  const fieldId = id ?? generated;
  return (
    <FieldShell {...{ label, optional, hint, error, icon, action, className }} id={fieldId}>
      <input
        id={fieldId}
        className={cn("l8-input", mono && "is-mono")}
        aria-invalid={error ? true : undefined}
        aria-describedby={hint || error ? `${fieldId}-hint` : undefined}
        {...input}
      />
    </FieldShell>
  );
}

interface TextAreaFieldProps extends Omit<TextareaHTMLAttributes<HTMLTextAreaElement>, "className"> {
  label?: string;
  optional?: string;
  hint?: ReactNode;
  error?: string | undefined;
  className?: string;
}

/** Multi-line variant of TextField. */
export function TextAreaField({ label, optional, hint, error, className, id, ...input }: TextAreaFieldProps) {
  const generated = useId();
  const fieldId = id ?? generated;
  return (
    <FieldShell {...{ label, optional, hint, error, className }} id={fieldId}>
      <textarea
        id={fieldId}
        className="l8-input"
        aria-invalid={error ? true : undefined}
        aria-describedby={hint || error ? `${fieldId}-hint` : undefined}
        {...input}
      />
    </FieldShell>
  );
}

interface SearchFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "className" | "size"> {
  size?: "sm" | "md";
  width?: number;
  shortcut?: string[];
  className?: string;
}

/** Pill search field with an optional keyboard hint. */
export function SearchField({ size = "md", width, shortcut, className, ...input }: SearchFieldProps) {
  return (
    <div className={cn("l8-search", size === "sm" && "l8-search-sm", className)} style={width ? { width } : undefined}>
      <DsIcon name="search" size={18} />
      <input type="search" aria-label={input.placeholder} {...input} />
      {shortcut ? (
        <span className="l8-keys">
          {shortcut.map((key) => (
            <kbd key={key}>{key}</kbd>
          ))}
        </span>
      ) : null}
    </div>
  );
}

interface CheckboxProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
  description?: string;
  disabled?: boolean;
  className?: string;
}

/** Checkbox row with an optional description. */
export function Checkbox({ checked, onChange, label, description, disabled, className }: CheckboxProps) {
  return (
    <label className={cn("l8-check", className)}>
      <button
        type="button"
        role="checkbox"
        aria-checked={checked}
        disabled={disabled}
        className={cn("l8-check-box", checked && "is-on")}
        onClick={() => onChange(!checked)}
      >
        {checked ? <DsIcon name="check" size={14} /> : null}
      </button>
      <span className="l8-check-text">
        <span className="l8-check-label">{label}</span>
        {description ? <span className="l8-check-desc">{description}</span> : null}
      </span>
    </label>
  );
}

export interface DropdownOption<T extends string | number> {
  id: T;
  label: string;
}

interface DropdownProps<T extends string | number> {
  value: T;
  options: readonly DropdownOption<T>[];
  onChange: (value: T) => void;
  /** Small caption before the value, e.g. "Sort". */
  prefix?: string;
  label?: string;
  disabled?: boolean;
  className?: string;
}

/** Dropdown with a floating menu. Esc or an outside click closes it. */
export function Dropdown<T extends string | number>({
  value,
  options,
  onChange,
  prefix,
  label,
  disabled,
  className,
}: DropdownProps<T>) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const current = options.find((option) => option.id === value);

  useEffect(() => {
    if (!open) return;
    const onPointer = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("pointerdown", onPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div ref={root} className={cn("l8-dd", open && "is-open", className)}>
      <button
        type="button"
        className="l8-dd-trigger"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={label}
        disabled={disabled}
        onClick={() => setOpen(!open)}
      >
        {prefix ? <span className="l8-dd-prefix">{prefix}</span> : null}
        <span className="l8-dd-value">{current?.label ?? ""}</span>
        <DsIcon name="chevronDown" size={16} />
      </button>
      {open ? (
        <div className="l8-menu l8-dd-menu" role="listbox">
          {options.map((option) => {
            const selected = option.id === value;
            return (
              <div
                key={String(option.id)}
                role="option"
                tabIndex={0}
                aria-selected={selected}
                className={cn("l8-menu-item", selected && "is-on")}
                onClick={() => {
                  onChange(option.id);
                  setOpen(false);
                }}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    onChange(option.id);
                    setOpen(false);
                  }
                }}
              >
                <span className="l8-menu-label">{option.label}</span>
                {selected ? <DsIcon name="check" size={16} className="l8-menu-check" /> : null}
              </div>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}

interface TabItem<T extends string> {
  id: T;
  label: string;
  count?: number;
}

interface TabsProps<T extends string> {
  value: T;
  items: readonly TabItem<T>[];
  onChange: (value: T) => void;
  tone?: "plain" | "stage";
  className?: string;
}

/** Pill tabs. `stage` sits on artwork. */
export function Tabs<T extends string>({ value, items, onChange, tone = "plain", className }: TabsProps<T>) {
  return (
    <div className={cn("l8-tabs", tone === "stage" && "l8-tabs-stage", className)} role="tablist">
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          role="tab"
          aria-selected={item.id === value}
          className={cn("l8-tab", item.id === value && "is-on")}
          onClick={() => onChange(item.id)}
        >
          {item.label}
          {item.count !== undefined ? <span className="l8-tab-count">{item.count}</span> : null}
        </button>
      ))}
    </div>
  );
}

interface OptionCardProps {
  selected: boolean;
  onClick: () => void;
  icon?: DsIconName;
  tone?: "cloud" | "local" | "flare" | "neutral";
  title: string;
  badge?: string;
  description?: ReactNode;
  meta?: string;
  disabled?: boolean;
  className?: string;
}

/** Selectable option card with radio semantics (dialogs and onboarding). */
export function OptionCard({
  selected,
  onClick,
  icon,
  tone,
  title,
  badge,
  description,
  meta,
  disabled,
  className,
}: OptionCardProps) {
  return (
    <button
      type="button"
      role="radio"
      aria-checked={selected}
      disabled={disabled}
      onClick={onClick}
      className={cn("l8-opt", selected && "is-on", disabled && "is-disabled", className)}
    >
      <span className="l8-opt-radio" />
      {icon ? (
        <span className={cn("l8-opt-icon", tone && `tone-${tone}`)}>
          <DsIcon name={icon} size={20} />
        </span>
      ) : null}
      <span className="l8-opt-text">
        <span className="l8-opt-title">
          {title}
          {badge ? <span className="l8-opt-badge">{badge}</span> : null}
        </span>
        {description ? <span className="l8-opt-desc">{description}</span> : null}
      </span>
      {meta ? <span className="l8-opt-meta">{meta}</span> : null}
    </button>
  );
}
