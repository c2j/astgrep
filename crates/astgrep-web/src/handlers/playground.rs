//! Interactive playground handler for testing the API

use axum::response::Html;
use crate::WebResult;

/// Interactive playground endpoint
pub async fn playground() -> WebResult<Html<String>> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>astgrep Playground</title>
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.8.0/styles/atom-one-light.min.css">
    <script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.8.0/highlight.min.js"></script>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            background: #f5f7fa;
            min-height: 100vh;
            padding: 0;
        }
        .container {
            max-width: 1600px;
            margin: 0 auto;
            height: 100vh;
            display: flex;
            flex-direction: column;
        }
        .header {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 20px 30px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
        }
        .header h1 { font-size: 1.8em; margin-bottom: 5px; }
        .header p { font-size: 0.95em; opacity: 0.9; }
        .main-content {
            display: flex;
            flex: 1;
            overflow: hidden;
            gap: 1px;
            background: #e0e0e0;
        }
        .panel {
            display: flex;
            flex-direction: column;
            background: white;
            overflow: hidden;
        }
        .left-panel {
            flex: 0 0 45%;
            border-right: 1px solid #e0e0e0;
        }
        .right-panel {
            flex: 1;
        }
        .panel-header {
            background: #f8f9fa;
            padding: 12px 16px;
            border-bottom: 1px solid #e0e0e0;
            font-weight: 600;
            color: #333;
            font-size: 13px;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        .panel-body {
            flex: 1;
            overflow-y: auto;
            padding: 16px;
        }
        .tabs {
            display: flex;
            gap: 0;
            border-bottom: 1px solid #e0e0e0;
            background: #f8f9fa;
        }
        .tab {
            padding: 10px 16px;
            cursor: pointer;
            border: none;
            background: none;
            font-size: 13px;
            color: #666;
            border-bottom: 2px solid transparent;
            transition: all 0.2s;
            font-weight: 500;
        }
        .tab:hover {
            background: #f0f0f0;
        }
        .tab.active {
            color: #667eea;
            border-bottom-color: #667eea;
            background: white;
        }
        .tab-content { display: none; }
        .tab-content.active { display: block; }
        .form-group {
            margin-bottom: 16px;
        }
        .form-group label {
            display: block;
            font-size: 12px;
            font-weight: 600;
            color: #333;
            margin-bottom: 6px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }
        textarea, select, input[type="file"] {
            width: 100%;
            padding: 10px;
            border: 1px solid #ddd;
            border-radius: 4px;
            font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
            font-size: 12px;
            background: white;
            color: #333;
        }
        textarea {
            resize: vertical;
            min-height: 250px;
            line-height: 1.5;
        }
        textarea:focus, select:focus, input:focus {
            outline: none;
            border-color: #667eea;
            box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1);
        }
        .button-group {
            display: flex;
            gap: 8px;
            margin-top: 16px;
        }
        .button-group button {
            flex: 1;
        }
        button {
            background: #667eea;
            color: white;
            padding: 10px 16px;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 13px;
            font-weight: 600;
            transition: all 0.2s;
        }
        button:hover {
            background: #5568d3;
            box-shadow: 0 2px 8px rgba(102, 126, 234, 0.3);
        }
        button:active {
            transform: translateY(1px);
        }
        button.secondary {
            background: #e9ecef;
            color: #333;
        }
        button.secondary:hover {
            background: #dee2e6;
        }
        .examples {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 8px;
            margin: 12px 0;
        }
        .example-btn {
            padding: 8px 12px;
            font-size: 11px;
            background: #f0f0f0;
            color: #333;
            border: 1px solid #ddd;
            border-radius: 3px;
            cursor: pointer;
            transition: all 0.2s;
        }
        .example-btn:hover {
            background: #e0e0e0;
            border-color: #ccc;
        }
        .code-editor {
            background: #fafafa;
            border: 1px solid #ddd;
            border-radius: 4px;
            overflow: hidden;
        }
        .code-editor pre {
            margin: 0;
            padding: 12px;
            overflow-x: auto;
            font-size: 12px;
            line-height: 1.5;
        }
        .code-editor code {
            font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
        }
        .results-section {
            margin-top: 16px;
        }
        .result-item {
            background: #f8f9fa;
            border: 1px solid #dee2e6;
            border-radius: 4px;
            padding: 12px;
            margin-bottom: 12px;
            font-size: 12px;
        }
        .result-item.error {
            background: #fff5f5;
            border-color: #feb2b2;
        }
        .result-item.warning {
            background: #fffaf0;
            border-color: #fbd38d;
        }
        .result-item.success {
            background: #f0fff4;
            border-color: #9ae6b4;
        }
        .result-header {
            font-weight: 600;
            margin-bottom: 8px;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        .result-content {
            font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
            white-space: pre-wrap;
            word-break: break-word;
            color: #333;
        }
        .stats {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 8px;
            margin-top: 12px;
            font-size: 12px;
        }
        .stat-item {
            background: #f0f0f0;
            padding: 8px;
            border-radius: 3px;
            text-align: center;
        }
        .stat-label {
            color: #666;
            font-size: 11px;
        }
        .stat-value {
            font-weight: 600;
            color: #333;
            font-size: 14px;
        }
        .loading {
            text-align: center;
            padding: 20px;
            color: #666;
        }
        .spinner {
            display: inline-block;
            width: 16px;
            height: 16px;
            border: 2px solid #f3f3f3;
            border-top: 2px solid #667eea;
            border-radius: 50%;
            animation: spin 1s linear infinite;
        }
        @keyframes spin {
            0% { transform: rotate(0deg); }
            100% { transform: rotate(360deg); }
        }
        .format-selector {
            display: flex;
            gap: 8px;
            margin-bottom: 12px;
        }
        .format-btn {
            flex: 1;
            padding: 8px;
            font-size: 12px;
            background: #f0f0f0;
            border: 1px solid #ddd;
            border-radius: 3px;
            cursor: pointer;
            transition: all 0.2s;
        }
        .format-btn.active {
            background: #667eea;
            color: white;
            border-color: #667eea;
        }
        /* Code editor with line numbers */
        .code-input-container {
            display: flex;
            border: 1px solid #ddd;
            border-radius: 4px;
            background: #fafafa;
            min-height: 300px;
        }
        .code-input-container .line-numbers {
            width: 44px;
            background: #f0f0f0;
            color: #999;
            padding: 10px 6px;
            text-align: right;
            font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
            font-size: 12px;
            line-height: 1.5;
            border-right: 1px solid #ddd;
            user-select: none;
            overflow: hidden;
        }
        .code-input-container textarea {
            border: none;
            outline: none;
            background: transparent;
            flex: 1;
            padding: 10px 10px 10px 8px;
            font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
            font-size: 12px;
            line-height: 1.5;
            resize: vertical;
        }

        @media (max-width: 1200px) {
            .main-content { flex-direction: column; }
            .left-panel { flex: 0 0 auto; border-right: none; border-bottom: 1px solid #e0e0e0; }
            .right-panel { flex: 1; }
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🔍 astgrep Playground</h1>
            <p>Interactive API Testing & Code Analysis</p>
        </div>

        <div class="main-content">
            <!-- Left Panel: YAML Rules Editor -->
            <div class="panel left-panel">
                <div class="tabs" id="left-tabs">
                    <button class="tab active" onclick="switchLeftTab('simple-tab', event)">simple</button>
                    <button class="tab" onclick="switchLeftTab('advanced-tab', event)">advanced</button>
                </div>

                <div class="panel-body">
                    <!-- Simple Tab -->
                    <div id="simple-tab" class="tab-content active">
                        <div class="form-group">
                            <label>Rule YAML</label>
                            <textarea id="rule-yaml" placeholder="Enter rule YAML..." style="min-height: 400px;">rules:
  - id: multiplication_rule
    pattern: $VAR1 * $VAR2;
    message: Use Math.pow(<number>, 2);
    languages:
      - javascript
    severity: INFO</textarea>
                        </div>
                    </div>

                    <!-- Advanced Tab -->
                    <div id="advanced-tab" class="tab-content">
                        <div class="form-group">
                            <label>Advanced Rule Configuration</label>
                            <textarea id="rule-advanced" placeholder="Advanced rule settings..." style="min-height: 400px;">rules:
  - id: advanced_multiplication_rule
    message: Consider using Math.pow() for better readability
    languages:
      - javascript
    severity: INFO
    confidence: HIGH
    metadata:
      cwe: CWE-1234
      owasp: A1
    patterns:
      - pattern-either:
          - pattern: $VAR1 * $VAR2
          - pattern: Math.pow($VAR1, 2)</textarea>
                        </div>
                    </div>
                </div>

                <!-- Inspect Rule Section -->
                <div id="inspect-rule-section" style="border-top: 1px solid #e0e0e0; padding: 12px 16px; background: #f8f9fa;">
                    <div style="font-weight: 600; color: #333; margin-bottom: 8px; font-size: 12px;">
                        ▼ Inspect Rule
                    </div>
                    <div id="inspect-rule-content" style="font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace; font-size: 11px; color: #666; line-height: 1.6;">
                        <div>pattern: $VAR1 * $VAR2;</div>
                    </div>
                </div>
            </div>

            <!-- Right Panel: Code & Results -->
            <div class="panel right-panel">
                <div class="tabs" id="right-tabs">
                    <button class="tab active" onclick="switchRightTab('test-code-tab', event)">test code</button>
                    <button class="tab" onclick="switchRightTab('metadata-tab', event)">metadata</button>
                    <button class="tab" onclick="switchRightTab('docs-tab', event)">docs</button>
                    <div style="flex: 1;"></div>
                </div>

                <div class="panel-body">
                    <!-- Test Code Tab -->
                    <div id="test-code-tab" class="tab-content active">
                        <div class="form-group">
                            <label>Language</label>
                            <div style="display:flex; gap:8px; align-items:center; margin-bottom: 12px;">
                                <select id="language" style="flex:1;">
                                    <option value="javascript">JavaScript</option>
                                    <option value="python">Python</option>
                                    <option value="java">Java</option>
                                    <option value="sql">SQL</option>
                                    <option value="bash">Bash</option>
                                    <option value="xml">XML</option>
                                    <option value="php">PHP</option>
                                    <option value="csharp">C#</option>
                                    <option value="c">C</option>
                                </select>
                                <button id="btn-load-example" class="secondary" title="Load example rule & code" onclick="loadExamplesForSelectedLanguage()">📋 Load Example</button>
                            </div>
                        </div>

                        <div class="form-group">
                            <label>Code</label>
                            <div class="code-input-container">
                                <div id="code-line-numbers" class="line-numbers"></div>
                                <textarea id="code-input" placeholder="Enter code to analyze..." style="min-height: 300px; background: #fafafa;" oninput="updateLineNumbers()" onscroll="syncCodeScroll()">// Prompt the user for a number
var userInput = prompt("Enter a number:");

// Convert the user input to a number
const number = parseFloat(userInput);

if (isNaN(number)) {
  // Calculate the square
  var square = number * number;
}

// Display the result
console.log('The square of ${number} is ${square}');
} else {
  console.log("Invalid input. Please enter a valid number.");
}</textarea>
                            </div>
                        </div>

                        <div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px;">
                            <button onclick="analyzeCode(event)" style="background: #4a90e2; color: white; padding: 10px 20px; border: none; border-radius: 4px; cursor: pointer; font-weight: 500; transition: background 0.2s;" onmouseover="this.style.background='#3a7bc8'" onmouseout="this.style.background='#4a90e2'">Run Ctrl+↵</button>
                            <button class="secondary" style="padding: 10px 12px; background: #f0f0f0; border: 1px solid #ddd; border-radius: 4px; cursor: pointer;" onclick="toggleRunMenu(event)">▼</button>
                        </div>

                        <!-- Results Section -->
                        <div style="margin-top: 20px; border-top: 1px solid #e0e0e0; padding-top: 16px;">
                            <div style="display:flex; justify-content: space-between; align-items: center; margin-bottom: 12px;">
                                <div style="font-weight: 600; color: #333; font-size: 13px;">Matches</div>
                                <div style="font-size: 12px; color: #666; display: flex; gap: 8px; align-items: center;">
                                    <span>排序:</span>
                                    <select id="sort-key" onchange="rerenderResults()" style="padding: 4px 6px; border: 1px solid #ddd; border-radius: 3px;">
                                        <option value="line" selected>行号</option>
                                        <option value="severity">严重等级</option>
                                        <option value="rule_id">规则ID</option>
                                    </select>
                                    <button id="sort-direction" class="secondary" onclick="toggleSortDirection()" title="升序/降序">↑</button>
                                </div>
                            </div>
                            <div id="results-content" style="font-size: 12px;">
                                <div style="color: #999; font-size: 12px;">Run analysis to see results</div>
                            </div>
                        </div>
                    </div>

                    <!-- Metadata Tab -->
                    <div id="metadata-tab" class="tab-content">
                        <div id="metadata-content" style="font-size: 12px; color: #666;">
                            <p>Run analysis to see metadata</p>
                        </div>
                    </div>

                    <!-- Docs Tab -->
                    <div id="docs-tab" class="tab-content">
                        <h3 style="margin-bottom: 12px;">API Documentation</h3>
                        <p style="font-size: 12px; line-height: 1.6; color: #666;">
                            <strong>POST /api/v1/analyze</strong><br>
                            Analyze code snippet for security issues.<br><br>
                            <strong>Request:</strong><br>
                            <code style="background: #f0f0f0; padding: 4px; border-radius: 3px;">
                            { "code": "...", "language": "javascript" }
                            </code><br><br>
                            <strong>Response:</strong><br>
                            Returns findings with severity, confidence, and location.
                        </p>
                    </div>
                </div>
            </div>
        </div>
    </div>
    <div id="status-bar" style="border-top: 1px solid #e0e0e0; background: #f8f9fa; padding: 8px 12px; font-size: 12px; color: #555; display: flex; justify-content: space-between; align-items: center;">
        <div id="status-text">Ready.</div>
        <div id="status-stats"></div>
    </div>


    <script>
        const API_BASE = 'http://127.0.0.1:8080/api/v1';
        let currentFormat = 'json';
        let currentMode = 'normal';
        let sortKey = 'line';
        let sortAsc = true;
        let lastFindings = [];
        let lastResponseData = null;
        let lastDurationMs = 0;

        // -------- Helpers: status, positions, line numbers, scrolling --------
        function setStatus(text, stats) {
            const statusText = document.getElementById('status-text');
            const statusStats = document.getElementById('status-stats');
            if (statusText) statusText.textContent = text || '';
            if (statusStats) statusStats.textContent = stats || '';
        }

        function normalizePos(f) {
            const loc = f.location || {};
            return {
                sL: Number(loc.start_line || f.start_line || f.line || 1) || 1,
                sC: Number(loc.start_column || f.start_column || 1) || 1,
                eL: Number(loc.end_line || f.end_line || loc.start_line || f.line || 1) || 1,
                eC: Number(loc.end_column || f.end_column || loc.start_column || 1) || 1,
            };
        }
        function comparePosition(a, b) {
            const A = normalizePos(a), B = normalizePos(b);
            if (A.sL !== B.sL) return A.sL - B.sL;
            if (A.sC !== B.sC) return A.sC - B.sC;
            if (A.eL !== B.eL) return A.eL - B.eL;
            return A.eC - B.eC;
        }
        function severityWeight(sev) {
            const s = (sev || '').toString().toLowerCase();
            const map = { critical: 4, high: 3, error: 3, warning: 2, medium: 2, low: 1, info: 0 };
            return map[s] ?? 0;
        }

        function updateLineNumbers() {
            const ta = document.getElementById('code-input');
            const ln = document.getElementById('code-line-numbers');
            if (!ta || !ln) return;
            const lines = ta.value.split('\n').length || 1;
            let html = '';
            for (let i = 1; i <= lines; i++) html += `<div>${i}</div>`;
            ln.innerHTML = html;
            syncCodeScroll();
        }
        function syncCodeScroll() {
            const ta = document.getElementById('code-input');
            const ln = document.getElementById('code-line-numbers');
            if (!ta || !ln) return;
            ln.scrollTop = ta.scrollTop;
        }
        function getIndexFromLineCol(text, line, col) {
            const L = Math.max(1, Number(line) || 1);
            const C = Math.max(1, Number(col) || 1);
            const arr = text.split('\n');
            const li = Math.min(L, arr.length);
            let idx = 0;
            for (let i = 0; i < li - 1; i++) idx += arr[i].length + 1; // +1 for \n
            const lineLen = arr[li - 1]?.length ?? 0;
            return idx + Math.min(lineLen, C - 1);
        }
        function jumpToLocation(sL, sC, eL, eC) {
            const ta = document.getElementById('code-input');
            if (!ta) return;
            const text = ta.value;
            const start = getIndexFromLineCol(text, sL, sC);
            const end = getIndexFromLineCol(text, eL || sL, eC || sC);
            try {
                ta.focus();
                ta.setSelectionRange(start, Math.max(start, end));
            } catch (_) {}
            // approximate scroll: 1.5 line-height * (line-1)
            const approxLineHeight = 18; // px (12px font-size * 1.5 line-height)
            ta.scrollTop = Math.max(0, (Math.max(1, sL) - 1) * approxLineHeight - ta.clientHeight / 3);
            syncCodeScroll();
        }

        function clearResults() {
            const content = document.getElementById('results-content');
            if (content) content.innerHTML = '<div style="color:#999; font-size:12px;">Run analysis to see results</div>';
            const metadata = document.getElementById('metadata-content');
            if (metadata) metadata.innerHTML = '<p>Run analysis to see metadata</p>';
            // 清除代码选择和滚动（避免上一轮高亮干扰）
            const ta = document.getElementById('code-input');
            if (ta) {
                try { ta.setSelectionRange(0, 0); } catch (_) {}
                ta.scrollTop = 0;
                syncCodeScroll();
            }
            lastFindings = [];
            lastResponseData = null;
            lastDurationMs = 0;
            setStatus('Ready.', '');
        }

        function toggleSortDirection() {
            sortAsc = !sortAsc;
            const btn = document.getElementById('sort-direction');
            if (btn) btn.textContent = sortAsc ? '↑' : '↓';
            rerenderResults();
        }

        function rerenderResults() {
            const content = document.getElementById('results-content');
            if (!content) return;
            const keySel = document.getElementById('sort-key');
            const key = keySel ? keySel.value : sortKey;
            sortKey = key;

            if (!lastFindings || lastFindings.length === 0) {
                content.innerHTML = '<div style="color:#999; font-size:12px;">No matches</div>';
                return;
            }

            const idxs = Array.from({ length: lastFindings.length }, (_, i) => i);
            idxs.sort((i, j) => {
                const A = lastFindings[i];
                const B = lastFindings[j];
                let cmp = 0;
                if (key === 'line') cmp = comparePosition(A, B);
                else if (key === 'severity') cmp = severityWeight(A.severity) - severityWeight(B.severity);
                else if (key === 'rule_id') cmp = String(A.rule_id || '').localeCompare(String(B.rule_id || ''));
                return sortAsc ? cmp : -cmp;
            });

            const severityColors = {
                critical: { bg: '#fff5f5', border: '#feb2b2', icon: '🔴' },
                high: { bg: '#fff5f5', border: '#feb2b2', icon: '🟠' },
                warning: { bg: '#fffaf0', border: '#fbd38d', icon: '🟡' },
                info: { bg: '#f0f7ff', border: '#b3d9ff', icon: '🔵' }
            };

            let html = '';
            for (const idx of idxs) {
                const f = lastFindings[idx];
                const pos = normalizePos(f);
                const sev = (f.severity || 'info').toLowerCase();
                const colors = severityColors[sev] || severityColors.info;
                const locText = `L${pos.sL}:C${pos.sC}` + (pos.eL !== pos.sL || pos.eC !== pos.sC ? ` → L${pos.eL}:C${pos.eC}` : '');
                html += `
                    <div style="background: ${colors.bg}; border: 1px solid ${colors.border}; border-radius: 4px; padding: 10px; margin-bottom: 10px;">
                        <div style="display:flex; justify-content: space-between; align-items:center; gap:8px;">
                            <div style="font-weight:600; color:#333;">${colors.icon} ${escapeHtml(f.message || 'No message')}</div>
                            <div style="display:flex; gap:6px;">
                                <button class="secondary" style="padding:2px 6px;" title="Jump" onclick="jumpToLocation(${pos.sL}, ${pos.sC}, ${pos.eL}, ${pos.eC})">🎯</button>
                                <button class="secondary" style="padding:2px 6px;" title="Copy" onclick="copyFinding(${idx})">📋</button>
                            </div>
                        </div>
                        <div style="font-size:11px; color:#666; margin-top:6px;">
                            <strong>Rule:</strong> ${escapeHtml(f.rule_id || 'N/A')} | <strong>Severity:</strong> ${sev.toUpperCase()} | <strong>Loc:</strong> ${locText}
                        </div>
                    </div>`;
            }

            // footer with stats
            const rulesExecuted = lastResponseData?.summary?.rules_executed;
            const filesAnalyzed = lastResponseData?.summary?.files_analyzed;
            html += `
                <div style="margin-top: 12px; padding-top: 10px; border-top: 1px solid #e0e0e0; display:flex; justify-content: space-between; color:#666; font-size:12px;">
                    <div>✓ ${lastFindings.length} match${lastFindings.length !== 1 ? 'es' : ''}</div>
                    <div>${filesAnalyzed ? filesAnalyzed + ' file' + (filesAnalyzed!==1?'s':'') + ' · ' : ''}${rulesExecuted ? rulesExecuted + ' rules · ' : ''}${lastDurationMs}ms</div>
                </div>`;

            content.innerHTML = html;
        }
        async function copyFinding(idx) {
            try {
                const f = lastFindings?.[idx];
                if (!f) return;
                const p = normalizePos(f);
                const text = [
                    `Rule: ${f.rule_id || 'N/A'}`,
                    `Severity: ${(f.severity || 'info').toUpperCase()}`,
                    `Location: L${p.sL}:C${p.sC}` + ((p.eL !== p.sL || p.eC !== p.sC) ? ` -> L${p.eL}:C${p.eC}` : ''),
                    `Message: ${f.message || ''}`
                ].join('\n');
                if (navigator.clipboard?.writeText) {
                    await navigator.clipboard.writeText(text);
                } else {
                    const ta = document.createElement('textarea');
                    ta.value = text; document.body.appendChild(ta); ta.select(); document.execCommand('copy'); document.body.removeChild(ta);
                }
                setStatus('Copied.', `${(f.rule_id || 'rule')} · L${p.sL}:C${p.sC}`);
            } catch (e) {
                console.error('copy failed', e);
                setStatus('Copy failed', '');
            }
        }

        function loadExamplesForSelectedLanguage() {
            const sel = document.getElementById('language');
            const lang = sel ? sel.value : 'javascript';
            const examples = {
                javascript: {
                    code: `function greet(name) {\n  eval('console.log(\"Hello, \" + name)');\n}\n\ngreet(userInput);`,
                    yaml: `rules:\n  - id: no_eval_js\n    message: Avoid eval(); use safer alternatives\n    languages:\n      - javascript\n    severity: WARNING\n    pattern: eval($X)`
                },
                python: {
                    code: `import pickle\n\nuser_data = input('> ')\nobj = pickle.loads(user_data)  # insecure deserialization` ,
                    yaml: `rules:\n  - id: insecure_pickle\n    message: Avoid pickle.loads on untrusted input\n    languages:\n      - python\n    severity: WARNING\n    pattern: pickle.loads($X)`
                },
                java: {
                    code: `public class Demo {\n  public static void run(String cmd) throws Exception {\n    Runtime.getRuntime().exec(cmd); // command injection risk\n  }\n}` ,
                    yaml: `rules:\n  - id: java_command_injection\n    message: Avoid Runtime.exec with untrusted input\n    languages:\n      - java\n    severity: WARNING\n    pattern: Runtime.getRuntime().exec($X)`
                },
                bash: {
                    code: `#!/usr/bin/env bash\ninput=$1\neval "$input"   # dangerous` ,
                    yaml: `rules:\n  - id: bash_eval\n    message: Avoid eval in shell scripts\n    languages:\n      - bash\n    severity: WARNING\n    pattern: eval $X`
                },
                sql: {
                    code: `SELECT * FROM users WHERE id = "' || user_id || '";` ,
                    yaml: `rules:\n  - id: select_star\n    message: Avoid SELECT *; specify columns\n    languages:\n      - sql\n    severity: WARNING\n    pattern: SELECT * FROM $TABLE`
                },
                xml: {
                    code: `<config>\n  <password>secret123</password>\n</config>` ,
                    yaml: `rules:\n  - id: xml_hardcoded_password\n    message: Avoid hardcoded passwords in XML\n    languages:\n      - xml\n    severity: WARNING\n    pattern: <password>$VALUE</password>`
                }
            };

            const ex = examples[lang] || examples.javascript;
            const yamlBox = document.getElementById('rule-yaml');
            const advBox = document.getElementById('rule-advanced');
            const codeBox = document.getElementById('code-input');

            if (yamlBox) yamlBox.value = ex.yaml;
            if (advBox && document.getElementById('advanced-tab')?.classList.contains('active')) advBox.value = ex.yaml;
            if (codeBox) codeBox.value = ex.code;

            updateLineNumbers();
            validateYAMLRule();
            clearResults();
            setStatus('Example loaded.', `${lang}`);
        }



        // 左侧面板tab切换
        function switchLeftTab(tabId, event) {
            if (event) {
                event.preventDefault();
                event.stopPropagation();
            }

            // 移除左侧所有tab的active状态
            document.querySelectorAll('#left-tabs .tab').forEach(el => el.classList.remove('active'));

            // 隐藏左侧所有tab内容
            document.querySelectorAll('.left-panel .tab-content').forEach(el => el.classList.remove('active'));

            // 激活选中的tab
            const tab = document.getElementById(tabId);
            if (tab) {
                tab.classList.add('active');
            }

            // 激活点击的按钮
            if (event && event.target) {
                event.target.classList.add('active');
            }

            // 更新Inspect Rule
            validateYAMLRule();
        }

        // 右侧面板tab切换
        function switchRightTab(tabId, event) {
            if (event) {
                event.preventDefault();
                event.stopPropagation();
            }

            // 移除右侧所有tab的active状态（排除mode按钮）
            document.querySelectorAll('#right-tabs .tab').forEach(el => {
                if (!el.id || (!el.id.startsWith('mode-'))) {
                    el.classList.remove('active');
                }
            });

            // 隐藏右侧所有tab内容
            document.querySelectorAll('.right-panel .tab-content').forEach(el => el.classList.remove('active'));

            // 激活选中的tab
            const tab = document.getElementById(tabId);
            if (tab) {
                tab.classList.add('active');
            }

            // 激活点击的按钮
            if (event && event.target) {
                event.target.classList.add('active');
            }
        }

        function setFormat(format) {
            currentFormat = format;
            const jsonBtn = document.getElementById('format-json');
            const sarifBtn = document.getElementById('format-sarif');
            if (jsonBtn) {
                jsonBtn.style.background = format === 'json' ? '#667eea' : '#f0f0f0';
                jsonBtn.style.color = format === 'json' ? 'white' : '#666';
            }
            if (sarifBtn) {
                sarifBtn.style.background = format === 'sarif' ? '#667eea' : '#f0f0f0';
                sarifBtn.style.color = format === 'sarif' ? 'white' : '#666';
            }
        }

        // mode removed

        function toggleRunMenu(event) {
            if (event) {
                event.preventDefault();
                event.stopPropagation();
            }
            // TODO: 实现运行选项菜单
            alert('Run options menu - to be implemented');
        }

        function showLoading() {
            const loading = document.getElementById('loading');
            const resultsContent = document.getElementById('results-content');

            if (loading) loading.style.display = 'block';
            if (resultsContent) resultsContent.innerHTML = '<div style="color: #999; font-size: 12px;">Analyzing...</div>';

            const statusText = document.getElementById('status-text');
            const statusStats = document.getElementById('status-stats');
            if (statusText) statusText.textContent = 'Analyzing...';
            if (statusStats) statusStats.textContent = '';
        }

        function hideLoading() {
            const loading = document.getElementById('loading');
            if (loading) {
                loading.style.display = 'none';
            }
        }

        function displayResults(data, startTime) {
            hideLoading();
            const duration = Date.now() - startTime;
            const content = document.getElementById('results-content');

            if (!data.findings || data.findings.length === 0) {
                content.innerHTML = `
                    <div class="result-item success">
                        <div class="result-header">✓ No issues found</div>
                        <div class="stats">
                            <div class="stat-item">
                                <div class="stat-label">Duration</div>
                                <div class="stat-value">${duration}ms</div>
                            </div>
                            <div class="stat-item">
                                <div class="stat-label">Rules</div>
                                <div class="stat-value">${data.summary?.rules_executed || 0}</div>
                            </div>
                        </div>
                    </div>
                `;
                return;
            }

            let html = '';
            data.findings.forEach((finding, idx) => {
                const severity = finding.severity || 'info';
                const icon = severity === 'critical' ? '🔴' : severity === 'high' ? '🟠' : severity === 'warning' ? '🟡' : '🔵';
                html += `
                    <div class="result-item ${severity === 'critical' || severity === 'high' ? 'error' : 'warning'}">
                        <div class="result-header">
                            <span>${icon} ${finding.message}</span>
                            <span style="font-size: 11px; color: #999;">Line ${finding.location?.line || '?'}</span>
                        </div>
                        <div class="result-content" style="margin-bottom: 8px;">
                            <strong>Rule:</strong> ${finding.rule_id || 'N/A'}<br>
                            <strong>Severity:</strong> ${severity.toUpperCase()}<br>
                            <strong>Confidence:</strong> ${finding.confidence || 'N/A'}
                        </div>
                    </div>
                `;
            });

            if (data.summary) {
                html += `
                    <div style="margin-top: 16px; padding-top: 16px; border-top: 1px solid #e0e0e0;">
                        <div class="stats">
                            <div class="stat-item">
                                <div class="stat-label">Total Findings</div>
                                <div class="stat-value">${data.summary.total_findings || 0}</div>
                            </div>
                            <div class="stat-item">
                                <div class="stat-label">Duration</div>
                                <div class="stat-value">${data.summary.duration_ms || duration}ms</div>
                            </div>
                            <div class="stat-item">
                                <div class="stat-label">Files</div>
                                <div class="stat-value">${data.summary.files_analyzed || 1}</div>
                            </div>
                            <div class="stat-item">
                                <div class="stat-label">Rules</div>
                                <div class="stat-value">${data.summary.rules_executed || 0}</div>
                            </div>
                        </div>
                    </div>
                `;
            }

            content.innerHTML = html;

            // Update metadata tab
            const metadata = document.getElementById('metadata-content');
            metadata.innerHTML = `<pre style="font-size: 11px; overflow-x: auto;">${JSON.stringify(data, null, 2)}</pre>`;
        }

        async function analyzeCode(event) {
            if (event) {
                event.preventDefault();
                event.stopPropagation();
            }

            const code = document.getElementById('code-input').value;
            const language = document.getElementById('language').value;

            if (!code.trim()) {
                alert('Please enter code to analyze');
                return;
            }

            // Validate YAML rule first
            if (!validateYAMLRule()) {
                alert('Please fix the rule errors first');
                return;
            }

            // 获取当前的YAML规则
            const simpleTab = document.getElementById('simple-tab');
            const advancedTab = document.getElementById('advanced-tab');
            let yamlRule = '';

            if (simpleTab && simpleTab.classList.contains('active')) {
                yamlRule = document.getElementById('rule-yaml').value;
            } else if (advancedTab && advancedTab.classList.contains('active')) {
                yamlRule = document.getElementById('rule-advanced').value;
            } else {
                yamlRule = document.getElementById('rule-yaml').value;
            }

            showLoading();
            const startTime = Date.now();

            try {
                const endpoint = currentFormat === 'sarif' ? '/analyze/sarif' : '/analyze';
                const requestBody = {
                    code,
                    language,
                    rules: yamlRule // 发送YAML规则
                };

                const response = await fetch(`${API_BASE}${endpoint}`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(requestBody)
                });

                const data = await response.json();
                if (response.ok) {
                    displayEnhancedResults(data, startTime);
                } else {
                    hideLoading();
                    const errMsg = escapeHtml(data.error || 'Analysis failed');
                    document.getElementById('results-content').innerHTML = `
                        <div style="background: #fff5f5; border: 1px solid #feb2b2; border-radius: 4px; padding: 12px;">
                            <div style="font-weight: 600; color: #d32f2f; margin-bottom: 4px;">❌ Error</div>
                            <div style="font-size: 12px; color: #333;">${errMsg}</div>
                        </div>
                    `;
                    setStatus('Error.', errMsg);
                }
            } catch (error) {
                hideLoading();
                const errText = escapeHtml(error.message);
                document.getElementById('results-content').innerHTML = `
                    <div style="background: #fff5f5; border: 1px solid #feb2b2; border-radius: 4px; padding: 12px;">
                        <div style="font-weight: 600; color: #d32f2f; margin-bottom: 4px;">❌ Error</div>
                        <div style="font-size: 12px; color: #333;">${errText}</div>
                    </div>
                `;
                setStatus('Error.', errText);
            }
        }

        async function analyzeSARIF() {
            setFormat('sarif');
            analyzeCode();
        }

        async function analyzeFile() {
            const file = document.getElementById('file-input').files[0];
            const language = document.getElementById('file-language').value;

            if (!file) {
                alert('Please select a file');
                return;
            }

            showLoading();
            const startTime = Date.now();

            const formData = new FormData();
            formData.append('file', file);
            formData.append('language', language);

            try {
                const response = await fetch(`${API_BASE}/analyze/file`, {
                    method: 'POST',
                    body: formData
                });

                const data = await response.json();
                if (response.ok) {
                    displayResults(data, startTime);
                } else {
                    hideLoading();
                    document.getElementById('results-content').innerHTML = `
                        <div class="result-item error">
                            <div class="result-header">❌ Error</div>
                            <div class="result-content">${data.error || 'Upload failed'}</div>
                        </div>
                    `;
                }
            } catch (error) {
                hideLoading();
                document.getElementById('results-content').innerHTML = `
                    <div class="result-item error">
                        <div class="result-header">❌ Error</div>
                        <div class="result-content">${error.message}</div>
                    </div>
                `;
            }
        }

        function loadExample(type) {
            const examples = {
                'js-eval': { code: 'function unsafe(input) { return eval(input); }', lang: 'javascript' },
                'py-pickle': { code: 'import pickle\ndata = pickle.loads(user_input)', lang: 'python' },
                'sql-injection': { code: 'SELECT * FROM users WHERE id = " + userId + "', lang: 'sql' },
                'java-sql': { code: 'String query = "SELECT * FROM users WHERE id = " + userId;', lang: 'java' }
            };

            const example = examples[type];
            if (example) {
                document.getElementById('code-input').value = example.code;
                document.getElementById('language').value = example.lang;
            }
        }

        // YAML 规则验证和解析
        function validateYAMLRule() {
            // 获取当前激活的tab
            const simpleTab = document.getElementById('simple-tab');
            const advancedTab = document.getElementById('advanced-tab');

            let yaml = '';
            if (simpleTab && simpleTab.classList.contains('active')) {
                yaml = document.getElementById('rule-yaml').value;
            } else if (advancedTab && advancedTab.classList.contains('active')) {
                yaml = document.getElementById('rule-advanced').value;
            } else {
                // 默认使用simple tab
                yaml = document.getElementById('rule-yaml').value;
            }

            if (!yaml.trim()) {
                showInspectRuleError('Please enter a rule');
                return false;
            }

            try {
                // 简单的 YAML 验证
                const lines = yaml.split('\n');
                let hasRules = false;
                let hasPattern = false;
                let hasId = false;
                let patterns = [];

                for (const line of lines) {
                    if (line.includes('rules:')) hasRules = true;
                    if (line.includes('id:')) hasId = true;

                    // 提取所有pattern
                    const patternMatch = line.match(/^\s*-?\s*pattern:\s*(.+)/);
                    if (patternMatch) {
                        hasPattern = true;
                        patterns.push(patternMatch[1].trim());
                    }
                }

                if (!hasRules && !hasPattern) {
                    showInspectRuleError('Missing "rules:" section or pattern');
                    return false;
                }

                if (!hasId && hasRules) {
                    showInspectRuleError('Missing "id:" field');
                    return false;
                }

                if (!hasPattern) {
                    showInspectRuleError('Missing "pattern:" field');
                    return false;
                }

                // 更新 Inspect Rule 显示
                if (patterns.length > 0) {
                    updateInspectRule(patterns);
                }

                return true;
            } catch (error) {
                showInspectRuleError('Invalid YAML: ' + error.message);
                return false;
            }
        }

        function updateInspectRule(patterns) {
            const inspectContent = document.getElementById('inspect-rule-content');
            if (inspectContent) {
                if (Array.isArray(patterns)) {
                    const html = patterns.map(p => `<div>pattern: ${escapeHtml(p)}</div>`).join('');
                    inspectContent.innerHTML = html || '<div style="color: #999;">No patterns found</div>';
                } else {
                    inspectContent.innerHTML = `<div>pattern: ${escapeHtml(patterns)}</div>`;
                }
            }
        }

        function showInspectRuleError(message) {
            const inspectContent = document.getElementById('inspect-rule-content');
            if (inspectContent) {
                inspectContent.innerHTML = `<div style="color: #d32f2f;">❌ ${escapeHtml(message)}</div>`;
            }
        }

        function escapeHtml(text) {
            const map = {
                '&': '&amp;',
                '<': '&lt;',
                '>': '&gt;',
                '"': '&quot;',
                "'": '&#039;'
            };
            return text.replace(/[&<>"']/g, m => map[m]);
        }

        // 监听 YAML 输入变化
        document.addEventListener('DOMContentLoaded', function() {
            const ruleYaml = document.getElementById('rule-yaml');
            const ruleAdvanced = document.getElementById('rule-advanced');

            if (ruleYaml) ruleYaml.addEventListener('input', validateYAMLRule);
            if (ruleAdvanced) ruleAdvanced.addEventListener('input', validateYAMLRule);

            // 初始化 Inspect Rule
            validateYAMLRule();

            // 初始化行号
            updateLineNumbers();

            // 初始化排序方向箭头
            const sortDirBtn = document.getElementById('sort-direction');
            if (sortDirBtn) sortDirBtn.textContent = '↑';
        });

        // 改进的结果显示
        function displayEnhancedResults(data, startTime) {
            hideLoading();
            const duration = Date.now() - startTime;
            lastDurationMs = duration;
            lastResponseData = data || null;

            const content = document.getElementById('results-content');
            if (!content) {
                console.error('results-content element not found');
                return;
            }

            // 统一提取 findings
            const findings = (data && (data.results?.findings || data.findings)) || [];

            // 更新元数据
            const metadata = document.getElementById('metadata-content');
            if (metadata) {
                metadata.innerHTML = `<pre style="font-size: 11px; overflow-x: auto; color: #333;">${JSON.stringify(data, null, 2)}</pre>`;
            }

            if (!Array.isArray(findings) || findings.length === 0) {
                content.innerHTML = `
                    <div style="background: #f0fff4; border: 1px solid #9ae6b4; border-radius: 4px; padding: 12px; margin-bottom: 12px;">
                        <div style="font-weight: 600; color: #22863a; margin-bottom: 4px;">✓ No issues found</div>
                        <div style="font-size: 11px; color: #666;">All checks passed successfully</div>
                    </div>
                `;
                setStatus('Complete.', `0 matches · ${duration}ms`);
                lastFindings = [];
                return;
            }

            // 先按位置信息排序（默认）
            const defaultSorted = [...findings].sort(comparePosition);
            lastFindings = defaultSorted;

            // 根据当前排序控件重新渲染
            rerenderResults();

            // 更新状态栏
            const rulesExecuted = data?.summary?.rules_executed;
            const filesAnalyzed = data?.summary?.files_analyzed;
            const statsParts = [];
            statsParts.push(`${findings.length} match${findings.length !== 1 ? 'es' : ''}`);
            if (filesAnalyzed) statsParts.push(`${filesAnalyzed} file${filesAnalyzed !== 1 ? 's' : ''}`);
            if (rulesExecuted) statsParts.push(`${rulesExecuted} rules`);
            statsParts.push(`${duration}ms`);
            setStatus('Complete.', statsParts.join(' · '));
        }

        // 键盘快捷键支持
        document.addEventListener('keydown', function(e) {
            // Ctrl+Enter 或 Cmd+Enter 运行分析
            if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
                e.preventDefault();
                analyzeCode(null);
            }
        });
    </script>
</body>
</html>"#;

    Ok(Html(html.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_playground() {
        let result = playground().await;
        assert!(result.is_ok());

        let html = result.unwrap().0;
        assert!(html.contains("astgrep Playground"));
        assert!(html.contains("analyzeCode"));
    }
}

