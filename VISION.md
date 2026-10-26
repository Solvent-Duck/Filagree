# Filagree: Vision

Draft, 2026-10-26. Demo quality only, not a product. Unknowns are marked [TBD].

## The demo

You ask an agent out loud: "How does our product manage data processing in Snowflake?" The agent reads a prepared repo, builds a Visualyze model and walkthrough, and the 3D viewer opens. From then on you drive with your hands: a gesture moves to the next step and the agent's narration for that step is spoken aloud; a closed fist spins the assembly; moving your hand toward or away from the camera zooms. No keyboard after the first question.

Everything on stage is prepared in advance: the repo, the question, and a checked fallback model, walkthrough and narration.

## How it fits together

```
mic ──> dictation ──> agent (Claude Code or CoCo)
                         │  reads demo repo, writes .visualyze/{model,walkthrough,narration}.json
                         └─ visualyze MCP ──> viewer (Bevy, --serve)
                                                 ▲  local socket: control_playback, camera
camera ──> gesture bridge (MediaPipe) ───────────┘
                │  on step change: speak narration[step_id]
                └─> TTS ──> speakers
```

- **Viewer** stays the one source of what's on screen. The agent still supplies semantics only.
- **Gesture bridge** is a separate local process. It talks to the viewer over the same per-user socket the MCP bridge uses (hello + token, then requests).
- **Narration** is a sidecar file the agent writes, keyed by walkthrough step ID. The viewer never sees prose; the bridge speaks it when the step changes, whether the change came from a gesture, a key or the agent.

## Gestures

MediaPipe's built-in gesture recognizer gives one label per frame plus palm position and size. Four actions, set after the first live trial:

| Gesture | Action |
| --- | --- |
| Open palm facing the camera, turned like a dial | Rotate |
| Fist, moved toward/away from the camera | Zoom |
| One finger up, swiped to your left | Next step |
| One finger up, swiped to your right | Previous step |

Labels survive recognition dropouts of up to 250 ms. A swipe is 15% of the frame width within 400 ms, then 1.5 s where finger motion is ignored, so the hand can return. Rotate and zoom are continuous, smoothed with a One Euro filter.

## What we build

1. **Viewer: a `camera` socket method.** Orbit and zoom deltas into the existing `CameraRig::adjust`, paused while a step transition plays. Small change in `private/runtime/src/{ipc,camera}.rs`.
2. **Gesture bridge (`gestures/`, Python).** Fresh copy of the pieces needed from AirCam (`~/Downloads/untitled folder`): camera capture, the MediaPipe `GestureSource`, latest-state buffer, One Euro filter, the pinned `gesture_recognizer.task`. New: gesture-to-action mapping, the viewer socket client, step-change polling and TTS.
3. **Narration sidecar.** `.visualyze/narration.json` maps step ID to one or two spoken sentences. Add it to the demo skill so the agent writes it with the walkthrough.
4. **Voice in.** Dictation into the agent's prompt: macOS Dictation or OpenWhispr (`~/Projects/openwhispr`) [TBD].
5. **Voice out.** macOS `say` with a premium voice first; a better local or hosted voice only if `say` sounds bad on stage [TBD].
6. **Demo repo.** Reshape `demo/acme-data` around Snowflake processing (Snowpipe, streams and tasks, dynamic tables, a Snowpark job, masking). About 12 to 16 boxes, real subsystems so steps can open them.
7. **Fallback and rehearsal.** Checked model, walkthrough and narration; a one-command reset; a run sheet.

## Not doing

- A web viewer or a new renderer
- Custom gesture training; MediaPipe's built-in labels only
- Gaze, two-hand gestures, pointer control
- Wake word or always-listening voice
- Generating narration inside the viewer
- Real company code or data in the demo repo
- Tests, CI or anything for production

## Risks

| Risk | Plan |
| --- | --- |
| AirCam's live inference runs under `sandbox-exec` with all networking denied, which may also block the viewer's Unix socket | Drop the sandbox in the copy; it's a demo |
| False gestures while talking with your hands | Hold times, neutral-before-repeat, and a "gestures off" gesture or key |
| Stage lighting hurts hand tracking | Rehearse in the room; a keyboard fallback is always live (`.` `,`) |
| Agent writes a different model live than rehearsed | Seeded repo plus the fallback files; ask the agent to present the prepared walkthrough if needed |
| CoCo can't speak or doesn't support hooks | Speech lives in the gesture bridge, not the agent, so it works with either |

## Open questions

- Which agent is on stage: Claude Code, CoCo, or both back to back? [TBD]
- Does the agent's first chat answer get spoken, or only per-step narration? [TBD]
- Laptop's built-in camera or an external one? [TBD]
