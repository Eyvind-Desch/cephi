import clsx from "clsx";
import { ArrowUpRight, ChevronRight, Loader2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { api, type SecretGroup } from "../../lib/ipc";
import { BrandIcon } from "../ui/BrandIcon";
import { guides } from "./guides";
import { SecretField } from "./SecretField";

type TestState = { status: "idle" } | { status: "testing" } | { status: "ok" | "error"; message: string };

interface SecretGroupRowProps {
  group: SecretGroup;
  open: boolean;
  onToggle: () => void;
  onChange: () => void;
}

export function SecretGroupRow({ group, open, onToggle, onChange }: SecretGroupRowProps) {
  const { t } = useTranslation();
  const [test, setTest] = useState<TestState>({ status: "idle" });
  const setCount = group.secrets.filter((s) => s.isSet).length;
  const complete = setCount === group.secrets.length;
  const Guide = guides[group.id];

  const runTest = async () => {
    setTest({ status: "testing" });
    try {
      setTest({ status: "ok", message: await api.secretsTest(group.id) });
    } catch (error) {
      setTest({ status: "error", message: String(error) });
    }
  };

  const status = complete
    ? t("keys.status.active")
    : setCount > 0
      ? t("keys.status.partial", { count: group.secrets.length - setCount })
      : t("keys.status.notSet");

  return (
    <div>
      <button
        type="button"
        onClick={onToggle}
        aria-expanded={open}
        className="flex w-full items-center gap-3 px-3 py-2.5 text-left transition-colors hover:bg-sunken"
      >
        <span className="grid size-7 shrink-0 place-items-center rounded-lg bg-sunken text-muted">
          <BrandIcon name={group.icon} size={15} />
        </span>
        <span className="min-w-0 flex-1 truncate text-[13px] text-fg">{group.title}</span>
        <span className={clsx("flex shrink-0 items-center gap-1.5 text-[12px]", complete ? "text-muted" : "text-subtle")}>
          <span
            className={clsx(
              "size-1.5 rounded-full",
              complete ? "bg-success" : setCount > 0 ? "bg-warning" : "border border-subtle",
            )}
          />
          {status}
        </span>
        <ChevronRight
          size={14}
          strokeWidth={1.75}
          className={clsx("shrink-0 text-subtle transition-transform duration-150", open && "rotate-90")}
        />
      </button>

      {/* Expands instantly: faster to use than an animated accordion. */}
      {open && (
            <div className="space-y-3 pt-0.5 pr-3 pb-3.5 pl-[52px]">
              <p className="text-[12px] leading-relaxed text-subtle">{group.description}</p>

              {Guide && <Guide />}

              <div className="space-y-3">
                {group.secrets.map((secret) => (
                  <SecretField key={secret.name} secret={secret} onChange={onChange} />
                ))}
              </div>

              {(group.testable || group.docsUrl) && (
                <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[12px]">
                  {group.testable && (
                    <button
                      type="button"
                      disabled={!complete || test.status === "testing"}
                      onClick={() => void runTest()}
                      className="flex items-center gap-1 text-muted transition-colors hover:text-fg disabled:pointer-events-none disabled:opacity-40"
                    >
                      {test.status === "testing" && <Loader2 size={12} className="animate-spin" />}
                      {t("keys.group.testConnection")}
                    </button>
                  )}
                  {group.docsUrl && (
                    <button
                      type="button"
                      onClick={() => void api.openExternal(group.docsUrl!)}
                      className="flex items-center gap-0.5 text-muted transition-colors hover:text-fg"
                    >
                      {t("keys.group.getKey")} <ArrowUpRight size={12} strokeWidth={1.75} />
                    </button>
                  )}
                  {(test.status === "ok" || test.status === "error") && (
                    <span className={clsx("basis-full", test.status === "ok" ? "text-success" : "text-danger")}>
                      {test.message}
                    </span>
                  )}
                </div>
              )}
            </div>
      )}
    </div>
  );
}
