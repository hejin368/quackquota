import type { SettingsSaveFeedback } from "../../hooks/useSettings";

interface Props {
  feedback: SettingsSaveFeedback;
  savingLabel: string;
  savedLabel: string;
  failedLabel: string;
}

/** A permanently reserved, non-blocking live region for settings persistence. */
export default function SettingsSaveFeedback({
  feedback,
  savingLabel,
  savedLabel,
  failedLabel,
}: Props) {
  const message =
    feedback.state === "saving"
      ? savingLabel
      : feedback.state === "saved"
        ? savedLabel
        : feedback.state === "error"
          ? failedLabel
          : "\u00a0";

  return (
    <div
      className="settings-save-feedback"
      data-state={feedback.state}
      role="status"
      aria-live="polite"
      aria-atomic="true"
    >
      {message}
    </div>
  );
}
