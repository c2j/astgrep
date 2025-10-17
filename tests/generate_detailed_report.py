#!/usr/bin/env python3
"""
Detailed Report Generator for CR-SemService
Generates comprehensive HTML and Markdown reports
"""

import json
from pathlib import Path
from datetime import datetime

class DetailedReportGenerator:
    def __init__(self, test_report_json):
        self.report_file = Path(test_report_json)
        with open(self.report_file, 'r') as f:
            self.data = json.load(f)

    def generate_html_report(self, output_file="test_report.html"):
        """Generate HTML report"""
        html = """
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CR-SemService Test Report</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; padding: 20px; }
        .container { max-width: 1200px; margin: 0 auto; background: white; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); padding: 30px; }
        h1 { color: #333; margin-bottom: 10px; }
        .subtitle { color: #666; margin-bottom: 30px; }
        .summary { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; margin-bottom: 30px; }
        .stat-card { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 20px; border-radius: 8px; }
        .stat-card.passed { background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%); }
        .stat-card.failed { background: linear-gradient(135deg, #eb3349 0%, #f45c43 100%); }
        .stat-card.skipped { background: linear-gradient(135deg, #fa709a 0%, #fee140 100%); }
        .stat-value { font-size: 32px; font-weight: bold; }
        .stat-label { font-size: 14px; opacity: 0.9; }
        table { width: 100%; border-collapse: collapse; margin-bottom: 30px; }
        th { background: #f8f9fa; padding: 12px; text-align: left; font-weight: 600; border-bottom: 2px solid #dee2e6; }
        td { padding: 12px; border-bottom: 1px solid #dee2e6; }
        tr:hover { background: #f8f9fa; }
        .badge { display: inline-block; padding: 4px 8px; border-radius: 4px; font-size: 12px; font-weight: 600; }
        .badge.pass { background: #d4edda; color: #155724; }
        .badge.fail { background: #f8d7da; color: #721c24; }
        .badge.skip { background: #fff3cd; color: #856404; }
        .progress-bar { width: 100%; height: 24px; background: #e9ecef; border-radius: 4px; overflow: hidden; }
        .progress-fill { height: 100%; background: linear-gradient(90deg, #11998e 0%, #38ef7d 100%); display: flex; align-items: center; justify-content: center; color: white; font-size: 12px; font-weight: bold; }
        .footer { margin-top: 30px; padding-top: 20px; border-top: 1px solid #dee2e6; color: #666; font-size: 12px; }
    </style>
</head>
<body>
    <div class="container">
        <h1>🧪 CR-SemService Test Report</h1>
        <p class="subtitle">Comprehensive Validation Report</p>
        
        <div class="summary">
"""
        
        # Add stat cards
        total = self.data["total_tests"]
        passed = self.data["passed"]
        failed = self.data["failed"]
        skipped = self.data["skipped"]
        pass_rate = (passed / total * 100) if total > 0 else 0
        
        html += f"""
            <div class="stat-card passed">
                <div class="stat-value">{passed}</div>
                <div class="stat-label">Passed ✅</div>
            </div>
            <div class="stat-card failed">
                <div class="stat-value">{failed}</div>
                <div class="stat-label">Failed ❌</div>
            </div>
            <div class="stat-card skipped">
                <div class="stat-value">{skipped}</div>
                <div class="stat-label">Skipped ⏭️</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">{pass_rate:.1f}%</div>
                <div class="stat-label">Pass Rate</div>
            </div>
        </div>
        
        <h2>Progress</h2>
        <div class="progress-bar">
            <div class="progress-fill" style="width: {pass_rate}%">{pass_rate:.1f}%</div>
        </div>
        
        <h2 style="margin-top: 30px;">Results by Suite</h2>
        <table>
            <thead>
                <tr>
                    <th>Suite</th>
                    <th>Total</th>
                    <th>Passed</th>
                    <th>Failed</th>
                    <th>Pass Rate</th>
                </tr>
            </thead>
            <tbody>
"""
        
        for suite, stats in self.data["test_suites"].items():
            total_suite = stats["passed"] + stats["failed"] + stats["skipped"]
            rate = (stats["passed"] / total_suite * 100) if total_suite > 0 else 0
            html += f"""
                <tr>
                    <td><strong>{suite}</strong></td>
                    <td>{total_suite}</td>
                    <td><span class="badge pass">{stats['passed']}</span></td>
                    <td><span class="badge fail">{stats['failed']}</span></td>
                    <td>{rate:.1f}%</td>
                </tr>
"""
        
        html += """
            </tbody>
        </table>
        
        <div class="footer">
            <p>Report generated: """ + datetime.now().strftime("%Y-%m-%d %H:%M:%S") + """</p>
            <p>CR-SemService Validation Suite</p>
        </div>
    </div>
</body>
</html>
"""
        
        with open(output_file, 'w') as f:
            f.write(html)
        
        print(f"📊 HTML report saved to {output_file}")

    def generate_text_report(self, output_file="test_report.txt"):
        """Generate plain text report"""
        with open(output_file, 'w') as f:
            f.write("="*70 + "\n")
            f.write("CR-SEMSERVICE TEST REPORT\n")
            f.write("="*70 + "\n\n")
            
            f.write(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n")
            
            # Summary
            f.write("SUMMARY\n")
            f.write("-"*70 + "\n")
            f.write(f"Total Tests: {self.data['total_tests']}\n")
            f.write(f"Passed: {self.data['passed']} ✅\n")
            f.write(f"Failed: {self.data['failed']} ❌\n")
            f.write(f"Skipped: {self.data['skipped']} ⏭️\n")
            
            if self.data['total_tests'] > 0:
                pass_rate = self.data['passed'] / self.data['total_tests'] * 100
                f.write(f"Pass Rate: {pass_rate:.1f}%\n")
            f.write("\n")
            
            # By Suite
            f.write("RESULTS BY SUITE\n")
            f.write("-"*70 + "\n")
            for suite, stats in self.data["test_suites"].items():
                total = stats["passed"] + stats["failed"] + stats["skipped"]
                if total > 0:
                    rate = stats["passed"] / total * 100
                    f.write(f"{suite}: {stats['passed']}/{total} ({rate:.1f}%)\n")
            f.write("\n")
            
            f.write("="*70 + "\n")
        
        print(f"📄 Text report saved to {output_file}")

if __name__ == "__main__":
    generator = DetailedReportGenerator("tests/test_report.json")
    generator.generate_html_report("tests/test_report.html")
    generator.generate_text_report("tests/test_report.txt")
    print("\n✅ Reports generated successfully!")

