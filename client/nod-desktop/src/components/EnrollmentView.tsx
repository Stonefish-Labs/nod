import { Check } from "lucide-react";
import { FormEvent, useState } from "react";
import { NOTIFICATION_SOUND_OPTIONS } from "../app/state";
import { canSubmitEnrollment } from "../domain";
import type { EnrollParams } from "../types";

interface EnrollmentViewProps {
  error: string | null;
  onEnroll: (params: EnrollParams) => Promise<void>;
}

export function EnrollmentView({
  error,
  onEnroll,
}: EnrollmentViewProps): JSX.Element {
  const [baseUrl, setBaseUrl] = useState("");
  const [deviceName, setDeviceName] = useState(navigator.platform || "Desktop");
  const [code, setCode] = useState("");
  const [notificationSound, setNotificationSound] = useState("default");

  const enrollmentDraft = {
    base_url: baseUrl,
    device_name: deviceName,
    code,
  };

  function submit(event: FormEvent<HTMLFormElement>): void {
    event.preventDefault();
    void onEnroll({
      ...enrollmentDraft,
      notification_sound: notificationSound,
    });
  }

  return (
    <main className="enrollment">
      <form className="enrollmentPanel" onSubmit={submit}>
        <h1>Nod</h1>
        <label>
          Server
          <input
            value={baseUrl}
            onChange={(event) => setBaseUrl(event.currentTarget.value)}
            placeholder="https://nod.example.com"
          />
        </label>
        <label>
          Device
          <input
            value={deviceName}
            onChange={(event) => setDeviceName(event.currentTarget.value)}
          />
        </label>
        <label>
          Code
          <input
            value={code}
            onChange={(event) => setCode(event.currentTarget.value.toUpperCase())}
            maxLength={16}
          />
        </label>
        <label>
          Sound
          <select
            value={notificationSound}
            onChange={(event) => setNotificationSound(event.currentTarget.value)}
          >
            {NOTIFICATION_SOUND_OPTIONS.map((option) => (
              <option key={option.id} value={option.id}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        {error ? <p className="formError">{error}</p> : null}
        <button type="submit" disabled={!canSubmitEnrollment(enrollmentDraft)}>
          <Check size={16} />
          Enroll
        </button>
      </form>
    </main>
  );
}
