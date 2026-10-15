import type { ReactNode } from "react";

interface IconButtonProps {
  active?: boolean;
  children: ReactNode;
  danger?: boolean;
  expanded?: boolean;
  label: string;
  onClick: () => void;
  pressed?: boolean;
  title: string;
}

export function IconButton({
  active = false,
  children,
  danger = false,
  expanded,
  label,
  onClick,
  pressed,
  title,
}: IconButtonProps) {
  return (
    <button
      className={"icon-button" + (active ? " is-active" : "") + (danger ? " is-danger" : "")}
      type="button"
      aria-label={label}
      aria-expanded={expanded}
      aria-pressed={pressed}
      title={title}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

export function CloseIcon() {
  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      <path d="m5.5 5.5 9 9M14.5 5.5l-9 9" />
    </svg>
  );
}
