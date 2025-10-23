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
                    <button id="mode-pro" class="tab" style="padding: 8px 12px; font-size: 12px; background: #f0f0f0; border: 1px solid #ddd; border-radius: 3px; margin-right: 8px; cursor: pointer;" onclick="setMode('pro', event)">Pro</button>
                    <button id="mode-turbo" class="tab" style="padding: 8px 12px; font-size: 12px; background: #f0f0f0; border: 1px solid #ddd; border-radius: 3px; cursor: pointer;" onclick="setMode('turbo', event)">Turbo</button>
                </div>

                <div class="panel-body">
                    <!-- Test Code Tab -->
                    <div id="test-code-tab" class="tab-content active">
                        <div class="form-group">
                            <label>Language</label>
                            <select id="language" style="margin-bottom: 12px;">
                                <option value="javascript">JavaScript</option>
                                <option value="python">Python</option>
                                <option value="java">Java</option>
                                <option value="sql">SQL</option>
                                <option value="bash">Bash</option>
                                <option value="php">PHP</option>
                                <option value="csharp">C#</option>
                                <option value="c">C</option>
                            </select>
                        </div>

                        <div class="form-group">
                            <label>Code</label>
                            <textarea id="code-input" placeholder="Enter code to analyze..." style="min-height: 300px; background: #fafafa; border: 1px solid #ddd;">// Prompt the user for a number
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

                        <div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px;">
                            <button onclick="analyzeCode(event)" style="background: #4a90e2; color: white; padding: 10px 20px; border: none; border-radius: 4px; cursor: pointer; font-weight: 500; transition: background 0.2s;" onmouseover="this.style.background='#3a7bc8'" onmouseout="this.style.background='#4a90e2'">Run Ctrl+↵</button>
                            <button class="secondary" style="padding: 10px 12px; background: #f0f0f0; border: 1px solid #ddd; border-radius: 4px; cursor: pointer;" onclick="toggleRunMenu(event)">▼</button>
                        </div>

                        <!-- Results Section -->
                        <div style="margin-top: 20px; border-top: 1px solid #e0e0e0; padding-top: 16px;">
                            <div style="font-weight: 600; color: #333; margin-bottom: 12px; font-size: 13px;">Matches</div>

                            <div id="results-content" style="font-size: 12px;">
                                <div style="background: #f0f7ff; border: 1px solid #b3d9ff; border-radius: 4px; padding: 12px; margin-bottom: 12px;">
                                    <div style="font-weight: 600; color: #0066cc; margin-bottom: 4px;">Line 9</div>
                                    <div style="color: #333; font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;">Use Math.pow(<number>, 2);</div>
                                </div>
                                <div style="background: #fffbf0; border: 1px solid #ffd699; border-radius: 4px; padding: 12px; margin-bottom: 12px;">
                                    <div style="color: #666; font-size: 11px;">No test assertions</div>
                                    <div style="color: #999; font-size: 11px; margin-top: 4px;">Add rule-id: <rule-id> above lines your rule should catch to signify a test.</div>
                                </div>
                                <div style="margin-top: 16px; padding-top: 12px; border-top: 1px solid #e0e0e0; display: flex; justify-content: space-between; align-items: center; font-size: 12px; color: #666;">
                                    <div>✓ 1 match</div>
                                    <div>Semgrep v1.41.0 · in 0.6s · ● tests passed ▼</div>
                                </div>
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

    <script>
        const API_BASE = 'http://127.0.0.1:8080/api/v1';
        let currentFormat = 'json';
        let currentMode = 'normal';

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

        function setMode(mode, event) {
            if (event) {
                event.preventDefault();
                event.stopPropagation();
            }
            currentMode = mode;

            // 更新按钮样式
            const proBtn = document.getElementById('mode-pro');
            const turboBtn = document.getElementById('mode-turbo');

            if (proBtn) {
                proBtn.style.background = mode === 'pro' ? '#667eea' : '#f0f0f0';
                proBtn.style.color = mode === 'pro' ? 'white' : '#333';
            }
            if (turboBtn) {
                turboBtn.style.background = mode === 'turbo' ? '#667eea' : '#f0f0f0';
                turboBtn.style.color = mode === 'turbo' ? 'white' : '#333';
            }

            console.log('Mode set to:', mode);
        }

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

            if (loading) {
                loading.style.display = 'block';
            }
            if (resultsContent) {
                resultsContent.innerHTML = '<div style="color: #999; font-size: 12px;">Analyzing...</div>';
            }
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
                    rules: yamlRule,  // 发送YAML规则
                    options: {
                        mode: currentMode
                    }
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
                    document.getElementById('results-content').innerHTML = `
                        <div style="background: #fff5f5; border: 1px solid #feb2b2; border-radius: 4px; padding: 12px;">
                            <div style="font-weight: 600; color: #d32f2f; margin-bottom: 4px;">❌ Error</div>
                            <div style="font-size: 12px; color: #333;">${escapeHtml(data.error || 'Analysis failed')}</div>
                        </div>
                    `;
                }
            } catch (error) {
                hideLoading();
                document.getElementById('results-content').innerHTML = `
                    <div style="background: #fff5f5; border: 1px solid #feb2b2; border-radius: 4px; padding: 12px;">
                        <div style="font-weight: 600; color: #d32f2f; margin-bottom: 4px;">❌ Error</div>
                        <div style="font-size: 12px; color: #333;">${escapeHtml(error.message)}</div>
                    </div>
                `;
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

            if (ruleYaml) {
                ruleYaml.addEventListener('input', validateYAMLRule);
            }
            if (ruleAdvanced) {
                ruleAdvanced.addEventListener('input', validateYAMLRule);
            }

            // 初始化 Inspect Rule
            validateYAMLRule();

            // 初始化mode按钮
            setMode('normal', null);
        });

        // 改进的结果显示
        function displayEnhancedResults(data, startTime) {
            hideLoading();
            const duration = Date.now() - startTime;
            const content = document.getElementById('results-content');

            if (!content) {
                console.error('results-content element not found');
                return;
            }

            // 调试：打印完整的响应数据
            console.log('🔍 Full response data:', JSON.stringify(data, null, 2));
            console.log('🔍 data.results:', data.results);
            console.log('🔍 data.results?.findings:', data.results?.findings);
            console.log('🔍 data.findings:', data.findings);

            // 从响应中提取findings
            const findings = data.results?.findings || data.findings || [];
            console.log('🔍 Extracted findings:', findings);

            if (findings.length === 0) {
                content.innerHTML = `
                    <div style="background: #f0fff4; border: 1px solid #9ae6b4; border-radius: 4px; padding: 12px; margin-bottom: 12px;">
                        <div style="font-weight: 600; color: #22863a; margin-bottom: 4px;">✓ No issues found</div>
                        <div style="font-size: 11px; color: #666;">All checks passed successfully</div>
                    </div>
                `;
                return;
            }

            let html = '';
            const severityColors = {
                'critical': { bg: '#fff5f5', border: '#feb2b2', icon: '🔴' },
                'high': { bg: '#fff5f5', border: '#feb2b2', icon: '🟠' },
                'warning': { bg: '#fffaf0', border: '#fbd38d', icon: '🟡' },
                'info': { bg: '#f0f7ff', border: '#b3d9ff', icon: '🔵' }
            };

            findings.forEach((finding, idx) => {
                const severity = (finding.severity || 'info').toLowerCase();
                const colors = severityColors[severity] || severityColors['info'];
                const line = finding.location?.start_line || finding.line || '?';

                html += `
                    <div style="background: ${colors.bg}; border: 1px solid ${colors.border}; border-radius: 4px; padding: 12px; margin-bottom: 12px;">
                        <div style="font-weight: 600; color: #333; margin-bottom: 4px;">
                            ${colors.icon} Line ${line}
                        </div>
                        <div style="color: #333; font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace; font-size: 12px; margin-bottom: 8px;">
                            ${escapeHtml(finding.message || 'No message')}
                        </div>
                        <div style="font-size: 11px; color: #666;">
                            <strong>Rule:</strong> ${escapeHtml(finding.rule_id || 'N/A')} |
                            <strong>Severity:</strong> ${severity.toUpperCase()} |
                            <strong>Confidence:</strong> ${finding.confidence || 'N/A'}
                        </div>
                    </div>
                `;
            });

            // 添加统计信息
            html += `
                <div style="margin-top: 16px; padding-top: 12px; border-top: 1px solid #e0e0e0; display: flex; justify-content: space-between; align-items: center; font-size: 12px; color: #666;">
                    <div>✓ ${findings.length} match${findings.length !== 1 ? 'es' : ''}</div>
                    <div>astgrep · in ${duration}ms · ● tests passed ▼</div>
                </div>
            `;

            // 确保清空旧内容后再设置新内容
            content.innerHTML = '';
            content.innerHTML = html;

            // 更新元数据
            const metadata = document.getElementById('metadata-content');
            if (metadata) {
                metadata.innerHTML = `<pre style="font-size: 11px; overflow-x: auto; color: #333;">${JSON.stringify(data, null, 2)}</pre>`;
            }
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

