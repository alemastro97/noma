/** Browser dev mock for the voice extras (src-tauri/src/voice/commands.rs). */
export const NOT_HANDLED = Symbol("not handled");

export function handleVoice(cmd: string, _a: Record<string, unknown>): unknown {
  switch (cmd) {
    case "voice_recording_for_item":
      // No card has kept audio in the browser.
      return null;
    case "voice_recording_audio":
      return new ArrayBuffer(0);
    case "voice_command_undo":
      return null;
    default:
      return NOT_HANDLED;
  }
}
