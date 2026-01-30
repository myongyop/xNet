#!/bin/bash
# Phase 5 Verification Script

echo "=== Phase 5: Real AI Integration Verification ==="
echo ""

# 1. Check Ollama Server
echo "1. Checking Ollama Server..."
if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "✅ Ollama server is running"
else
    echo "❌ Ollama server not responding. Starting..."
    ./bin/ollama serve > ollama.log 2>&1 &
    sleep 3
fi

# 2. List Models
echo ""
echo "2. Available Models:"
./bin/ollama list

# 3. Test Runtime
echo ""
echo "3. Testing Runtime (Quick Inference)..."
curl -s -X POST http://localhost:11434/api/generate \
  -d '{"model": "tinyllama", "prompt": "Say hello in one word", "stream": false}' \
  | jq -r '.response' | head -c 50
echo "..."

# 4. Build Check
echo ""
echo ""
echo "4. Build Check..."
cargo check --quiet -p desktop && echo "✅ Desktop build OK" || echo "❌ Build failed"

echo ""
echo "=== Verification Complete ==="
echo ""
echo "Next Steps:"
echo "  1. Run Desktop App: cd desktop && npm run tauri dev"
echo "  2. Click 'Start Node'"
echo "  3. Check 'Available Models' section"
echo "  4. Test real inference via demo_chat.py"
