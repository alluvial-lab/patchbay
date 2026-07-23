import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const e2eRoot = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(e2eRoot, "..");
const adapterRoot = join(repoRoot, "pi-adapter");
const codingAgent = await import(
  pathToFileURL(
    join(adapterRoot, "node_modules/@earendil-works/pi-coding-agent/dist/index.js"),
  ).href
);
const fauxProvider = await import(
  pathToFileURL(
    join(adapterRoot, "node_modules/@earendil-works/pi-ai/dist/providers/faux.js"),
  ).href
);
const { AdapterProcess } = await import("../pi-adapter/dist/src/main.js");
const { PiSession } = await import("../pi-adapter/dist/src/pi_session.js");

const runtimeSessionId = process.env.WALKING_SESSION_ID ?? "walking-session";
const deploymentScope = process.env.WALKING_DEPLOYMENT_SCOPE ?? "walking-machine";
const provider = "patchbay-walking-skeleton";
const faux = fauxProvider.createFauxCore({ provider, api: provider, tokensPerSecond: 0 });
faux.setResponses([
  async () => {
    await delay(1_500);
    return fauxProvider.fauxAssistantMessage("Walking skeleton received the instruction");
  },
]);

let stopped = false;
process.once("SIGINT", () => {
  stopped = true;
});
process.once("SIGTERM", () => {
  stopped = true;
});

const adapter = new AdapterProcess({
  coreAddress: requiredEnv("PATCHBAY_CORE_ADDR"),
  adapterId: "pi",
  authorityDomainId: requiredEnv("PATCHBAY_AUTHORITY_DOMAIN_ID"),
  attachmentEvidence: requiredEnv("PATCHBAY_ADAPTER_ATTACHMENT_SECRET"),
  adapterGeneration: 1,
  sessions: [
    {
      cwd: repoRoot,
      runtimeSessionId,
      deploymentScope,
      project: "patchbay",
      name: "walking-skeleton",
      generation: 1,
    },
  ],
  createSession: async (configured) => {
    const auth = codingAgent.AuthStorage.inMemory({
      [provider]: { type: "api_key", key: "walking-skeleton" },
    });
    const registry = codingAgent.ModelRegistry.inMemory(auth);
    const model = faux.getModel();
    registry.registerProvider(provider, {
      name: "Patchbay walking-skeleton provider",
      apiKey: "walking-skeleton",
      baseUrl: "http://localhost:0",
      api: model.api,
      streamSimple: faux.streamSimple,
      models: [
        {
          id: model.id,
          name: model.name,
          api: model.api,
          baseUrl: "http://localhost:0",
          reasoning: model.reasoning,
          input: model.input,
          cost: model.cost,
          contextWindow: model.contextWindow,
          maxTokens: model.maxTokens,
        },
      ],
    });
    return PiSession.create({
      ...configured,
      model: `${provider}/${model.id}`,
      sessionOptions: {
        modelRegistry: registry,
        sessionManager: codingAgent.SessionManager.inMemory(repoRoot),
        settingsManager: codingAgent.SettingsManager.inMemory(),
        noTools: "all",
      },
    });
  },
});

try {
  await adapter.start();
  console.log(`PI_ADAPTER_READY ${runtimeSessionId}`);
  while (!stopped) {
    const delivered = await adapter.pollOnce();
    if (delivered > 0) console.log(`PI_ADAPTER_PROCESSED ${delivered}`);
    if (delivered === 0) await delay(50);
  }
} finally {
  await adapter.dispose();
}

function requiredEnv(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

function delay(milliseconds) {
  return new Promise((resolveDelay) => setTimeout(resolveDelay, milliseconds));
}
