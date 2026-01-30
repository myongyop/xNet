import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface SystemSpecs {
  cpu_cores: number;
  total_memory: number;
  role_recommendation: string;
}

interface NodeMetrics {
  uptime_seconds: number;
  tasks_processed: number;
  tasks_relayed: number;
  credits: number;
}

function App() {
  const [onboardingComplete, setOnboardingComplete] = useState(false);
  const [specs, setSpecs] = useState<SystemSpecs | null>(null);
  const [status, setStatus] = useState("Offline");
  const [logs, setLogs] = useState<string[]>([]);
  const [peers, setPeers] = useState<string[]>([]);
  const [currentMode, setCurrentMode] = useState("Nerve");
  const [models, setModels] = useState<string[]>([]);
  const [metrics, setMetrics] = useState({
    uptime_seconds: 0,
    tasks_processed: 0,
    tasks_relayed: 0,
    credits: 0.0,
  });
  const [bootnodeInput, setBootnodeInput] = useState("");

  const fetchModels = async () => {
    try {
      const modelList = await invoke<string[]>("list_models");
      setModels(modelList);
      addLog(`Found ${modelList.length} models: ${modelList.join(", ")}`);
    } catch (e) {
      console.error("Failed to fetch models:", e);
    }
  };

  useEffect(() => {
    invoke<SystemSpecs>("get_system_specs").then(setSpecs);

    const unlistenPeerConnect = listen<string>("peer-connected", (event) => {
      setPeers((prev) => [...prev, event.payload]);
      addLog(`Peer Connected: ${event.payload}`);
    });

    const unlistenPeerDisconnect = listen<string>("peer-disconnected", (event) => {
      setPeers((prev) => prev.filter((p) => p !== event.payload));
      addLog(`Peer Disconnected: ${event.payload}`);
    });

    const unlistenLog = listen<string>("log-message", (event) => {
      addLog(`Msg: ${event.payload}`);
    });

    const unlistenDht = listen<string>("dht-event", (event) => {
      addLog(`[DHT] ${event.payload}`);
    });

    const unlistenMode = listen<string>("mode-change", (event) => {
      setCurrentMode(event.payload);
      addLog(`[MODE] Switched to ${event.payload}`);
    });

    const unlistenMetrics = listen<NodeMetrics>("metrics-updated", (event) => {
      setMetrics(event.payload);
    });

    const unlistenPipeline = listen<any>("pipeline-event", (event) => {
      const payload = event.payload;
      let msg = "";
      if (payload.InitSession) msg = `[Pipeline] Init Session: ${payload.InitSession.session_id}`;
      else if (payload.ForwardPass) {
        const shape = payload.ForwardPass.tensor.shape.join("x");
        msg = `[Pipeline] Forward Pass: ${payload.ForwardPass.session_id} (Layer ${payload.ForwardPass.layer_start}) [Tensor: ${shape}]`;
      }
      else if (payload.Result) msg = `[Pipeline] Result: ${payload.Result.token}`;
      else if (payload.Error) msg = `[Pipeline] Error: ${payload.Error.error}`;
      else msg = `[Pipeline] Unknown Event: ${JSON.stringify(payload)}`;
      addLog(msg);
    });

    const unlistenVerification = listen<any>("verification-event", (event) => {
      const payload = event.payload;
      let msg = "";
      if (payload.ChallengeIssued) {
        const c = payload.ChallengeIssued;
        msg = `[Verification] Challenge Issued: Layer ${c.target_layer} for session ${c.target_session_id}`;
      }
      else if (payload.ProofSubmitted) msg = `[Verification] Proof Submitted for challenge`;
      else if (payload.ChallengeResolved) {
        const resolved = payload.ChallengeResolved;
        msg = `[Verification] Challenge ${resolved.valid ? "Passed" : "FAILED"}`;
      }
      addLog(msg);
    });

    const unlistenFL = listen<any>("fl-event", (event) => {
      const payload = event.payload;
      let msg = "";
      if (payload.LocalUpdate) {
        const u = payload.LocalUpdate;
        msg = `[FL] Local Update: ${u.node_id} (Round ${u.round}) - ${u.metrics}`;
      } else if (payload.GlobalModelUpdate) {
        msg = `[FL] Global Model Updated! (Round ${payload.GlobalModelUpdate.round})`;
      }
      addLog(msg);
    });

    return () => {
      unlistenPeerConnect.then((f) => f());
      unlistenPeerDisconnect.then((f) => f());
      unlistenLog.then((f) => f());
      unlistenDht.then((f) => f());
      unlistenMode.then((f) => f());
      unlistenMetrics.then((f) => f());
      unlistenPipeline.then((f) => f());
      unlistenVerification.then((f) => f());
      unlistenFL.then((f) => f());
    };
  }, []);

  const addLog = (msg: string) => {
    setLogs((prev) => [`[${new Date().toLocaleTimeString()}] ${msg}`, ...prev.slice(0, 19)]);
  };

  const startNode = async () => {
    try {
      setStatus("Starting...");
      const res = await invoke("start_node", { bootnode: bootnodeInput });
      setStatus("Online (" + res + ")");
      addLog("Node service started.");
      fetchModels();
    } catch (e) {
      setStatus("Error: " + e);
      addLog("Failed to start node: " + e);
    }
  };

  const formatUptime = (seconds: number) => {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = Math.floor(seconds % 60);
    return `${h}h ${m}m ${s}s`;
  };

  if (!onboardingComplete && specs) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-xnet-darker via-xnet-dark to-xnet-darker flex items-center justify-center p-8 animate-fade-in">
        <div className="max-w-2xl w-full glass rounded-2xl p-8 glow">
          <h1 className="text-4xl font-bold text-white mb-6 text-center bg-gradient-to-r from-xnet-purple to-purple-400 bg-clip-text text-transparent">
            Welcome to xNet
          </h1>
          <div className="space-y-6">
            <div className="glass rounded-xl p-6">
              <h2 className="text-xl font-semibold text-white mb-4">System Analysis</h2>
              <div className="space-y-3 text-gray-300">
                <div className="flex justify-between">
                  <span>CPU Cores:</span>
                  <span className="font-mono text-xnet-purple">{specs.cpu_cores}</span>
                </div>
                <div className="flex justify-between">
                  <span>Total Memory:</span>
                  <span className="font-mono text-xnet-purple">{(specs.total_memory / (1024 ** 3)).toFixed(2)} GB</span>
                </div>
                <div className="flex justify-between">
                  <span>Recommended Role:</span>
                  <span className={`font-semibold ${specs.role_recommendation === "Muscle" ? "text-xnet-success" : "text-xnet-info"}`}>
                    {specs.role_recommendation} Node
                  </span>
                </div>
              </div>
            </div>
            <p className="text-gray-400 text-center">
              {specs.role_recommendation === "Muscle"
                ? "Your powerful machine is ideal for AI Inference (Muscle Node)."
                : "Your device is perfect for Network Relay & Verification (Nerve Node)."}
            </p>
            <button
              onClick={() => setOnboardingComplete(true)}
              className="w-full bg-gradient-to-r from-xnet-purple to-purple-600 hover:from-purple-600 hover:to-xnet-purple text-white font-semibold py-3 px-6 rounded-xl transition-all transform hover:scale-105 active:scale-95"
            >
              Accept & Continue
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-xnet-darker via-xnet-dark to-xnet-darker p-6 animate-fade-in">
      <div className="max-w-7xl mx-auto space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <h1 className="text-3xl font-bold text-white">
            xNet Dashboard
            <span className="text-xnet-purple ml-2">({currentMode})</span>
          </h1>
          <div className={`flex items-center gap-2 px-4 py-2 rounded-full glass ${status.startsWith("Online") ? "glow" : ""}`}>
            <div className={`w-3 h-3 rounded-full ${status.startsWith("Online") ? "bg-xnet-success animate-pulse-slow" : "bg-xnet-error"}`}></div>
            <span className="text-white font-medium">{status}</span>
          </div>
        </div>

        {/* Metrics Grid */}
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          <div className="glass rounded-xl p-6 hover:glow transition-all transform hover:scale-105">
            <div className="text-gray-400 text-sm mb-2">Active Peers</div>
            <div className="text-3xl font-bold text-white">{peers.length}</div>
          </div>
          <div className="glass rounded-xl p-6 hover:glow transition-all transform hover:scale-105">
            <div className="text-gray-400 text-sm mb-2">Uptime</div>
            <div className="text-2xl font-bold text-white">{formatUptime(metrics.uptime_seconds)}</div>
          </div>
          <div className="glass rounded-xl p-6 hover:glow transition-all transform hover:scale-105">
            <div className="text-gray-400 text-sm mb-2">Tasks Processed</div>
            <div className="text-3xl font-bold text-xnet-success">{metrics.tasks_processed}</div>
          </div>
          <div className="glass rounded-xl p-6 hover:glow transition-all transform hover:scale-105 bg-gradient-to-br from-xnet-purple/20 to-purple-600/20">
            <div className="text-gray-400 text-sm mb-2">Credits</div>
            <div className="text-3xl font-bold text-xnet-purple">{metrics.credits.toFixed(2)}</div>
          </div>
        </div>

        {/* Control Panel */}
        <div className="glass rounded-xl p-6">
          <h2 className="text-xl font-semibold text-white mb-4">Controls</h2>
          <div className="space-y-4">
            <div className="flex gap-3">
              <input
                type="text"
                placeholder="Bootnode Address (Optional)"
                value={bootnodeInput}
                onChange={(e) => setBootnodeInput(e.target.value)}
                className="flex-1 bg-xnet-darker/50 border border-white/10 rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-xnet-purple"
              />
            </div>
            <div className="grid grid-cols-2 md:grid-cols-5 gap-3">
              <button
                onClick={startNode}
                disabled={status.startsWith("Online")}
                className="bg-xnet-success hover:bg-green-600 disabled:bg-gray-600 text-white font-semibold py-2 px-4 rounded-lg transition-all"
              >
                Start Node
              </button>
              <button
                onClick={() => invoke("test_pipeline_event")}
                disabled={!status.startsWith("Online")}
                className="bg-xnet-info hover:bg-blue-600 disabled:bg-gray-600 text-white font-semibold py-2 px-4 rounded-lg transition-all"
              >
                Test Pipeline
              </button>
              <button
                onClick={() => invoke("test_verification_event")}
                disabled={!status.startsWith("Online")}
                className="bg-xnet-error hover:bg-red-600 disabled:bg-gray-600 text-white font-semibold py-2 px-4 rounded-lg transition-all"
              >
                Test Challenge
              </button>
              <button
                onClick={() => invoke("test_fl_event")}
                disabled={!status.startsWith("Online")}
                className="bg-xnet-purple hover:bg-purple-600 disabled:bg-gray-600 text-white font-semibold py-2 px-4 rounded-lg transition-all"
              >
                Start Training
              </button>
            </div>
          </div>
        </div>

        {/* Available Models */}
        {models.length > 0 && (
          <div className="glass rounded-xl p-6">
            <h2 className="text-xl font-semibold text-white mb-4">📦 Available Models</h2>
            <div className="flex flex-wrap gap-2">
              {models.map((model, i) => (
                <div
                  key={i}
                  className="px-4 py-2 bg-xnet-purple/20 border border-xnet-purple rounded-lg text-xnet-purple font-medium hover:bg-xnet-purple/30 transition-all"
                >
                  {model}
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Logs */}
        <div className="glass rounded-xl p-6">
          <h2 className="text-xl font-semibold text-white mb-4">Network & Pipeline Logs</h2>
          <div className="bg-xnet-darker/50 rounded-lg p-4 h-64 overflow-y-auto font-mono text-sm space-y-1">
            {logs.length === 0 ? (
              <div className="text-gray-500 text-center py-8">No activity yet...</div>
            ) : (
              logs.map((log, i) => (
                <div
                  key={i}
                  className={`${log.includes("[Pipeline]")
                    ? "text-xnet-info"
                    : log.includes("[Verification]")
                      ? "text-xnet-error"
                      : log.includes("[FL]")
                        ? "text-xnet-purple"
                        : "text-gray-300"
                    }`}
                >
                  {log}
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
