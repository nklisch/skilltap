import { z } from "zod/v4";

const ErrorEvent = z.object({
  kind: z.literal("error"),
  message: z.string(),
  hint: z.string().optional(),
});

const ProgressStartEvent = z.object({ kind: z.literal("progress:start"), label: z.string() });
const ProgressUpdateEvent = z.object({ kind: z.literal("progress:update"), label: z.string(), message: z.string() });
const ProgressDoneEvent = z.object({ kind: z.literal("progress:done"), label: z.string(), message: z.string().optional() });
const ProgressFailEvent = z.object({ kind: z.literal("progress:fail"), label: z.string(), message: z.string().optional() });

export const InstallEventSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("install:start"), source: z.string() }),
  z.object({ kind: z.literal("install:placed"), name: z.string(), path: z.string() }),
  z.object({ kind: z.literal("install:captured"), pluginName: z.string(), skills: z.array(z.string()), mcpServers: z.array(z.string()) }),
  z.object({ kind: z.literal("install:done"), records: z.array(z.string()), pluginName: z.string().optional() }),
  ErrorEvent,
  ProgressStartEvent,
  ProgressUpdateEvent,
  ProgressDoneEvent,
  ProgressFailEvent,
]);

export const UpdateEventSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("update:start"), name: z.string() }),
  z.object({ kind: z.literal("update:up-to-date"), name: z.string() }),
  z.object({ kind: z.literal("update:updated"), name: z.string(), fromRef: z.string().nullable(), toRef: z.string().nullable() }),
  z.object({ kind: z.literal("update:skipped"), name: z.string(), reason: z.string() }),
  z.object({ kind: z.literal("update:done"), updated: z.array(z.string()), skipped: z.array(z.string()), upToDate: z.array(z.string()) }),
  ErrorEvent,
  ProgressStartEvent,
  ProgressUpdateEvent,
  ProgressDoneEvent,
  ProgressFailEvent,
]);

export const SyncEventSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("sync:plan"), inSync: z.boolean(), items: z.array(z.unknown()) }),
  z.object({ kind: z.literal("sync:item"), source: z.string(), status: z.enum(["ok", "skipped", "fail"]), error: z.string().optional() }),
  z.object({ kind: z.literal("sync:done"), inSync: z.boolean(), applied: z.number(), skipped: z.number(), failed: z.number() }),
  ErrorEvent,
]);

export const DoctorEventSchema = z.object({
  ok: z.boolean(),
  checks: z.array(
    z.object({
      name: z.string(),
      status: z.enum(["pass", "warn", "fail"]),
      detail: z.string().optional(),
      issues: z.array(z.unknown()).optional(),
      info: z.array(z.string()).optional(),
    }),
  ),
});

export const StatusEventSchema = z.object({
  projectRoot: z.string().nullable(),
  hasManifest: z.boolean(),
  scope: z.string(),
  fromV2State: z.boolean(),
  skills: z.array(z.unknown()),
  plugins: z.array(z.unknown()),
  taps: z.array(z.unknown()),
  drift: z.unknown().optional(),
});

export const FindEventSchema = z.unknown();
export const VerifyEventSchema = z.unknown();
export const TryEventSchema = z.unknown();
export const MigrateEventSchema = z.unknown();
export const InfoEventSchema = z.unknown();
export const TapListEventSchema = z.unknown();
export const TapInfoEventSchema = z.unknown();
export const ConfigGetEventSchema = z.unknown();
export const ToggleEventSchema = z.unknown();
export const EnableEventSchema = z.unknown();
export const DisableEventSchema = z.unknown();

export type InstallEvent = z.infer<typeof InstallEventSchema>;
export type UpdateEvent = z.infer<typeof UpdateEventSchema>;
export type SyncEvent = z.infer<typeof SyncEventSchema>;
export type DoctorEvent = z.infer<typeof DoctorEventSchema>;
export type StatusEvent = z.infer<typeof StatusEventSchema>;
