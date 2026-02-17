#!/usr/bin/env python3
"""
Test Analyzer for CR-SemService
Analyzes test results and generates detailed reports
"""

import json
import yaml
from pathlib import Path
from collections import defaultdict
from datetime import datetime

class TestAnalyzer:
    def __init__(self, test_report_file):
        self.report_file = Path(test_report_file)
        with open(self.report_file, 'r') as f:
            self.results = json.load(f)
        
        self.analysis = {
            "summary": {},
            "by_suite": {},
            "by_language": defaultdict(lambda: {"passed": 0, "failed": 0, "total": 0}),
            "by_pattern_type": defaultdict(lambda: {"passed": 0, "failed": 0, "total": 0}),
            "failure_analysis": [],
            "performance_metrics": {}
        }

    def analyze_results(self):
        """Analyze test results"""
        self._analyze_summary()
        self._analyze_by_suite()
        self._analyze_failures()
        self._analyze_performance()
        return self.analysis

    def _analyze_summary(self):
        """Analyze overall summary"""
        total = self.results["total_tests"]
        passed = self.results["passed"]
        failed = self.results["failed"]
        skipped = self.results["skipped"]
        
        self.analysis["summary"] = {
            "total_tests": total,
            "passed": passed,
            "failed": failed,
            "skipped": skipped,
            "pass_rate": (passed / total * 100) if total > 0 else 0,
            "skip_rate": (skipped / total * 100) if total > 0 else 0,
            "fail_rate": (failed / total * 100) if total > 0 else 0,
            "quality_score": self._calculate_quality_score(passed, failed, total)
        }

    def _analyze_by_suite(self):
        """Analyze results by test suite"""
        for suite, stats in self.results["test_suites"].items():
            total = stats["passed"] + stats["failed"] + stats["skipped"]
            if total > 0:
                self.analysis["by_suite"][suite] = {
                    "total": total,
                    "passed": stats["passed"],
                    "failed": stats["failed"],
                    "skipped": stats["skipped"],
                    "pass_rate": (stats["passed"] / total * 100),
                    "test_count": len(stats.get("tests", []))
                }

    def _analyze_failures(self):
        """Analyze failure patterns"""
        failures = []
        for suite, stats in self.results["test_suites"].items():
            for test in stats.get("tests", []):
                if test["result"]["status"] == "failed":
                    failures.append({
                        "test": test["name"],
                        "suite": suite,
                        "reason": test["result"].get("reason", "Unknown"),
                        "stderr": test["result"].get("stderr", "")[:200]
                    })
        
        self.analysis["failure_analysis"] = failures

    def _analyze_performance(self):
        """Analyze performance metrics"""
        start = datetime.fromisoformat(self.results["start_time"])
        end = datetime.fromisoformat(self.results["end_time"])
        duration = (end - start).total_seconds()
        
        total_tests = self.results["total_tests"]
        avg_time = duration / total_tests if total_tests > 0 else 0
        
        self.analysis["performance_metrics"] = {
            "total_duration_seconds": duration,
            "average_test_time_seconds": avg_time,
            "tests_per_second": total_tests / duration if duration > 0 else 0
        }

    def _calculate_quality_score(self, passed, failed, total):
        """Calculate overall quality score (0-100)"""
        if total == 0:
            return 0
        
        pass_rate = passed / total
        fail_rate = failed / total
        
        # Quality score: 100 * pass_rate - 50 * fail_rate
        score = (100 * pass_rate) - (50 * fail_rate)
        return max(0, min(100, score))

    def generate_markdown_report(self, output_file="test_report.md"):
        """Generate markdown report"""
        with open(output_file, 'w') as f:
            f.write("# CR-SemService Test Report\n\n")
            
            # Summary
            f.write("## 📊 Summary\n\n")
            summary = self.analysis["summary"]
            f.write(f"- **Total Tests**: {summary['total_tests']}\n")
            f.write(f"- **Passed**: {summary['passed']} ✅\n")
            f.write(f"- **Failed**: {summary['failed']} ❌\n")
            f.write(f"- **Skipped**: {summary['skipped']} ⏭️\n")
            f.write(f"- **Pass Rate**: {summary['pass_rate']:.1f}%\n")
            f.write(f"- **Quality Score**: {summary['quality_score']:.1f}/100\n\n")
            
            # By Suite
            f.write("## 📈 Results by Suite\n\n")
            f.write("| Suite | Total | Passed | Failed | Pass Rate |\n")
            f.write("|-------|-------|--------|--------|----------|\n")
            for suite, stats in self.analysis["by_suite"].items():
                f.write(f"| {suite} | {stats['total']} | {stats['passed']} | {stats['failed']} | {stats['pass_rate']:.1f}% |\n")
            f.write("\n")
            
            # Failures
            if self.analysis["failure_analysis"]:
                f.write("## ❌ Failures\n\n")
                for failure in self.analysis["failure_analysis"]:
                    f.write(f"### {failure['test']}\n")
                    f.write(f"- **Suite**: {failure['suite']}\n")
                    f.write(f"- **Reason**: {failure['reason']}\n")
                    if failure['stderr']:
                        f.write(f"- **Error**: {failure['stderr']}\n")
                    f.write("\n")
            
            # Performance
            f.write("## ⚡ Performance\n\n")
            perf = self.analysis["performance_metrics"]
            f.write(f"- **Total Duration**: {perf['total_duration_seconds']:.2f}s\n")
            f.write(f"- **Average Test Time**: {perf['average_test_time_seconds']:.3f}s\n")
            f.write(f"- **Tests per Second**: {perf['tests_per_second']:.1f}\n\n")
        
        print(f"📄 Markdown report saved to {output_file}")

    def print_analysis(self):
        """Print analysis to console"""
        print("\n" + "="*70)
        print("TEST ANALYSIS REPORT")
        print("="*70)
        
        summary = self.analysis["summary"]
        print(f"\n📊 Overall Summary:")
        print(f"  Total Tests: {summary['total_tests']}")
        print(f"  Passed: {summary['passed']} ({summary['pass_rate']:.1f}%)")
        print(f"  Failed: {summary['failed']} ({summary['fail_rate']:.1f}%)")
        print(f"  Skipped: {summary['skipped']} ({summary['skip_rate']:.1f}%)")
        print(f"  Quality Score: {summary['quality_score']:.1f}/100")
        
        print(f"\n📈 Results by Suite:")
        for suite, stats in self.analysis["by_suite"].items():
            print(f"  {suite}: {stats['passed']}/{stats['total']} ({stats['pass_rate']:.1f}%)")
        
        if self.analysis["failure_analysis"]:
            print(f"\n❌ Top Failures:")
            for failure in self.analysis["failure_analysis"][:5]:
                print(f"  - {failure['test']}: {failure['reason']}")
        
        perf = self.analysis["performance_metrics"]
        print(f"\n⚡ Performance:")
        print(f"  Total Duration: {perf['total_duration_seconds']:.2f}s")
        print(f"  Avg Test Time: {perf['average_test_time_seconds']:.3f}s")
        print(f"  Tests/Second: {perf['tests_per_second']:.1f}")
        
        print("\n" + "="*70)

if __name__ == "__main__":
    analyzer = TestAnalyzer("tests/test_report.json")
    analyzer.analyze_results()
    analyzer.print_analysis()
    analyzer.generate_markdown_report("tests/test_report.md")

