# astgrep-dataflow

Data flow analysis, taint tracking, call graph, constant propagation, and symbolic analysis.

## Structure

```
src/
├── lib.rs                    # DataFlowAnalyzer — orchestrates graph build → source/sink/sanitizer detection → taint track
├── graph.rs                  # DataFlowGraph, DataFlowNode, NodeId, EdgeType (ControlFlow/DataFlow)
├── sources.rs                # SourceDetector — identifies taint sources (user input, HTTP params, env vars)
├── sinks.rs                  # SinkDetector — identifies dangerous sinks (SQL exec, eval, file write)
├── sanitizers.rs             # SanitizerDetector — identifies sanitization functions (escape, parameterize)
├── taint.rs                  # TaintTracker — basic source→sink flow tracking
├── enhanced_taint.rs         # EnhancedTaintTracker — advanced flow with inter-procedural awareness
├── advanced_taint.rs         # Advanced taint analysis with deeper path sensitivity
├── flows.rs                  # TaintFlow, FlowPath — represents a taint propagation path
├── call_graph.rs             # Call graph construction and analysis
├── interprocedural.rs        # Inter-procedural data flow (cross-function tracking)
├── symbol_table.rs           # Symbol table for variable tracking across scopes
├── constant_propagation/     # Compile-time constant value tracking
│   ├── mod.rs                # ConstantPropagator
│   ├── analysis.rs           # Propagation algorithm
│   ├── state.rs              # Abstract state for propagation
│   └── utils.rs              # Helper functions
├── constant_analysis.rs      # ConstantValue enum, constant expression evaluation
└── symbolic_propagation.rs   # Symbolic value propagation
```

## Where to Look

| Task | File | Notes |
|------|------|-------|
| Add new taint source | `sources.rs` | Add pattern to `SourceDetector` |
| Add new dangerous sink | `sinks.rs` | Add pattern to `SinkDetector` |
| Add sanitizer | `sanitizers.rs` | Register in `SanitizerDetector` |
| Taint flow algorithm | `taint.rs` → `enhanced_taint.rs` → `advanced_taint.rs` | Escalating complexity |
| Constant folding | `constant_propagation/analysis.rs` | Forward data flow analysis |
| Cross-function tracking | `interprocedural.rs` + `call_graph.rs` | Call graph → flow across boundaries |
| Symbol resolution | `symbol_table.rs` | Scoped variable → value mapping |
| Main entry point | `lib.rs` → `DataFlowAnalyzer::analyze()` | Returns `DataFlowAnalysis` |

## Key Types

- `DataFlowAnalyzer` — main orchestrator; builds graph, runs all analyses
- `DataFlowAnalysis` — result container (graph + sources + sinks + sanitizers + taint_flows + constant_values)
- `DataFlowGraph` — directed graph with `EdgeType::{ControlFlow, DataFlow}`
- `TaintFlow` — a source→sink path, with `is_vulnerable()` check
- `ConstantValue` — enum for tracked constant values
- `Source` / `Sink` / `Sanitizer` — tagged AST nodes with type metadata

## Conventions

- Analysis pipeline: `build_graph()` → detect sources/sinks/sanitizers → `track_taint()` → constant propagation
- Three tiers of taint analysis: basic (`taint.rs`) → enhanced (`enhanced_taint.rs`) → advanced (`advanced_taint.rs`)
- `DataFlowAnalysis::has_vulnerable_flows()` is the primary boolean check for vulnerability presence
- `DataFlowAnalysis::vulnerable_flows()` returns all flows where taint reaches a sink without sanitization
- Constant propagation is optional — enabled via `RuleContext::enable_constant_propagation` in astgrep-rules
