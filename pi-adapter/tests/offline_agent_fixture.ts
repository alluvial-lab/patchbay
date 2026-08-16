import { resolve } from "node:path";
import {
  DefaultResourceLoader,
  ModelRuntime,
  SessionManager,
  SettingsManager,
  type ResourceLoader,
} from "@earendil-works/pi-coding-agent";
import {
  InMemoryCredentialStore,
  InMemoryModelsStore,
} from "@earendil-works/pi-ai";
import {
  registerOfflineFixtureModelRuntime,
  type AgentSessionRuntimeFixtureServices,
  type OfflineFixtureModelRuntime,
} from "../src/pi_session.js";

export async function createOfflineModelRuntime(): Promise<OfflineFixtureModelRuntime> {
  const runtime = await ModelRuntime.create({
    credentials: new InMemoryCredentialStore(),
    modelsStore: new InMemoryModelsStore(),
    modelsPath: null,
    refreshOnCreate: false,
    allowModelNetwork: false,
  });
  return registerOfflineFixtureModelRuntime(runtime);
}

export async function createOfflineFixtureServices(
  cwd: string,
  modelRuntime: OfflineFixtureModelRuntime,
  sessionManager = SessionManager.inMemory(cwd),
): Promise<AgentSessionRuntimeFixtureServices> {
  const settingsManager = SettingsManager.inMemory({
    compaction: { enabled: false },
    retry: { enabled: false },
  });
  const resourceLoader: ResourceLoader = new DefaultResourceLoader({
    cwd,
    agentDir: resolve(cwd, ".patchbay-test-agent"),
    settingsManager,
    noExtensions: true,
    noSkills: true,
    noPromptTemplates: true,
    noThemes: true,
    noContextFiles: true,
  });
  await resourceLoader.reload();
  return {
    modelRuntime,
    resourceLoader,
    sessionManager,
    settingsManager,
    modelCatalogAuthStub: { kind: "offline-injected" },
  };
}
