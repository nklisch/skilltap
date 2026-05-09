export { loadManifest, manifestExists } from "./load";
export { loadLockfile, lockfileExists, saveLockfile } from "./lockfile";
export {
  LOCKFILE_FILENAME,
  lockfilePath,
  MANIFEST_FILENAME,
  manifestPath,
  PUBLISH_DIR,
  publishDir,
} from "./paths";
export { discoverPublishablePlugins, type PublishDiscovery } from "./publish";
export * from "./range";
export { recoverLockfile, recoverManifest } from "./recover";
export { saveManifest } from "./save";
export * from "./schemas";
export {
  addMcpToLockfile,
  addMcpToManifest,
  addPluginToManifest,
  addSkillToManifest,
  canonicalizeSourceKey,
  type LockfileMcpUpdateInput,
  type ManifestMcpUpdateInput,
  type ManifestUpdateInput,
  removeMcpFromLockfile,
  removeMcpFromManifest,
  removePluginFromManifest,
  removeSkillFromManifest,
  setManifestComponentActive,
} from "./update";
