import type { CodeEditorProps } from "@cloudscape-design/components/code-editor";

/**
 * Stands in for Cloudscape's CodeEditor in jsdom, where Ace does not run
 * (unit-test-instructions: the same value and change notices, a plain text
 * field). Shortcut keys are handled by QueryEditor around it, so they are
 * checked on this field too.
 */
export default function CodeEditor({ value, onChange, ariaLabel, loading }: CodeEditorProps) {
  return (
    <textarea
      aria-label={ariaLabel}
      aria-busy={loading ? "true" : undefined}
      value={value}
      onChange={(event) =>
        onChange?.(
          new CustomEvent("change", { detail: { value: event.target.value } }) as Parameters<
            NonNullable<CodeEditorProps["onChange"]>
          >[0],
        )
      }
      data-testid="code-editor"
    />
  );
}
