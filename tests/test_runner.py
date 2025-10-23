#!/usr/bin/env python3
"""
测试执行框架 - 批量运行semgrep和astgrep并比较结果
"""

import os
import json
import subprocess
import time
from pathlib import Path
from dataclasses import dataclass
from typing import List, Dict, Optional, Tuple
import tempfile

@dataclass
class TestCase:
    """测试用例数据结构"""
    rule_file: str
    target_file: str
    directory: str
    language: str

@dataclass 
class TestResult:
    """测试结果数据结构"""
    test_case: TestCase
    semgrep_output: Optional[str]
    cr_semservice_output: Optional[str]
    semgrep_findings: List[Dict]
    cr_semservice_findings: List[Dict]
    semgrep_error: Optional[str]
    cr_semservice_error: Optional[str]
    execution_time_semgrep: float
    execution_time_cr: float
    is_consistent: bool
    differences: List[str]

class TestRunner:
    """测试运行器"""
    
    def __init__(self, workspace_root: str = "."):
        self.workspace_root = Path(workspace_root)
        self.tests_dir = self.workspace_root / "tests"
        self.results = []
        
    def discover_test_cases(self, max_cases: int = 50) -> List[TestCase]:
        """发现测试用例"""
        test_cases = []
        
        # 语言扩展名映射
        language_extensions = {
            '.java': 'java',
            '.py': 'python', 
            '.js': 'javascript',
            '.ts': 'typescript',
            '.go': 'go',
            '.c': 'c',
            '.cpp': 'cpp',
            '.cs': 'csharp',
            '.php': 'php',
            '.rb': 'ruby',
            '.rs': 'rust',
            '.scala': 'scala',
            '.kt': 'kotlin',
            '.swift': 'swift'
        }
        
        # 优先级目录（按重要性排序）
        priority_dirs = [
            "taint_maturity",
            "tainting_rules", 
            "rules",
            "patterns",
            "explanations",
            "irrelevant_rules"
        ]
        
        for priority_dir in priority_dirs:
            if len(test_cases) >= max_cases:
                break
                
            dir_path = self.tests_dir / priority_dir
            if not dir_path.exists():
                continue
                
            # 递归搜索该目录下的测试用例
            for root, dirs, files in os.walk(dir_path):
                if len(test_cases) >= max_cases:
                    break
                    
                root_path = Path(root)
                yaml_files = [f for f in files if f.endswith(('.yaml', '.yml'))]
                
                for yaml_file in yaml_files:
                    if len(test_cases) >= max_cases:
                        break
                        
                    yaml_path = root_path / yaml_file
                    
                    # 查找对应的目标文件
                    base_name = yaml_file.rsplit('.', 1)[0]
                    
                    # 在同一目录下查找匹配的目标文件
                    for target_file in files:
                        if target_file == yaml_file:
                            continue
                            
                        target_base = target_file.rsplit('.', 1)[0]
                        target_ext = '.' + target_file.rsplit('.', 1)[1] if '.' in target_file else ''
                        
                        # 检查是否是匹配的目标文件
                        if (target_base == base_name and 
                            target_ext in language_extensions):
                            
                            test_case = TestCase(
                                rule_file=str(yaml_path),
                                target_file=str(root_path / target_file),
                                directory=str(root_path.relative_to(self.tests_dir)),
                                language=language_extensions[target_ext]
                            )
                            test_cases.append(test_case)
                            break
        
        print(f"发现 {len(test_cases)} 个测试用例")
        return test_cases
    
    def run_semgrep(self, rule_file: str, target_file: str) -> Tuple[Optional[str], Optional[str], float]:
        """运行semgrep"""
        start_time = time.time()
        try:
            cmd = ["semgrep", "--config", rule_file, target_file, "--json"]
            result = subprocess.run(
                cmd, 
                capture_output=True, 
                text=True, 
                timeout=30,
                cwd=self.workspace_root
            )
            execution_time = time.time() - start_time
            
            if result.returncode == 0:
                return result.stdout, None, execution_time
            else:
                return None, result.stderr, execution_time
                
        except subprocess.TimeoutExpired:
            return None, "Timeout", time.time() - start_time
        except Exception as e:
            return None, str(e), time.time() - start_time
    
    def run_cr_semservice(self, rule_file: str, target_file: str) -> Tuple[Optional[str], Optional[str], float]:
        """运行astgrep"""
        start_time = time.time()
        try:
            cmd = [
                "cargo", "run", "--bin", "astgrep", 
                "analyze", "--config", rule_file, target_file
            ]
            result = subprocess.run(
                cmd, 
                capture_output=True, 
                text=True, 
                timeout=60,
                cwd=self.workspace_root
            )
            execution_time = time.time() - start_time
            
            if result.returncode == 0:
                return result.stdout, None, execution_time
            else:
                return None, result.stderr, execution_time
                
        except subprocess.TimeoutExpired:
            return None, "Timeout", time.time() - start_time
        except Exception as e:
            return None, str(e), time.time() - start_time
    
    def parse_semgrep_output(self, output: str) -> List[Dict]:
        """解析semgrep输出"""
        try:
            data = json.loads(output)
            return data.get('results', [])
        except:
            return []
    
    def parse_cr_semservice_output(self, output: str) -> List[Dict]:
        """解析astgrep输出"""
        try:
            # astgrep输出可能包含日志，需要提取JSON部分
            lines = output.strip().split('\n')

            # 寻找JSON块的开始和结束
            json_start = -1
            json_end = -1
            brace_count = 0

            for i, line in enumerate(lines):
                line = line.strip()
                if line.startswith('{') and json_start == -1:
                    json_start = i
                    brace_count = 1
                elif json_start != -1:
                    brace_count += line.count('{') - line.count('}')
                    if brace_count == 0:
                        json_end = i
                        break

            if json_start != -1 and json_end != -1:
                # 提取JSON内容
                json_lines = lines[json_start:json_end+1]
                json_content = '\n'.join(json_lines)

                data = json.loads(json_content)
                findings = data.get('findings', [])
                print(f"    解析到JSON: {len(findings)} 个发现")
                return findings
            else:
                print(f"    未找到完整的JSON块")
                return []

        except Exception as e:
            print(f"    解析astgrep输出失败: {e}")
            # 尝试简单的方法：查找包含findings的行
            try:
                if '"findings"' in output:
                    # 查找最后一个包含findings的JSON对象
                    import re
                    pattern = r'\{[^{}]*"findings"[^{}]*\[[^\]]*\][^{}]*\}'
                    matches = re.findall(pattern, output, re.DOTALL)
                    if matches:
                        data = json.loads(matches[-1])
                        return data.get('findings', [])
            except:
                pass
            return []
    
    def compare_results(self, semgrep_findings: List[Dict], cr_findings: List[Dict]) -> Tuple[bool, List[str]]:
        """比较两个工具的结果"""
        differences = []
        
        # 比较发现数量
        if len(semgrep_findings) != len(cr_findings):
            differences.append(f"发现数量不同: semgrep={len(semgrep_findings)}, astgrep={len(cr_findings)}")
        
        # 简单的位置比较（这里可以进一步优化）
        semgrep_locations = set()
        cr_locations = set()
        
        for finding in semgrep_findings:
            if 'start' in finding and 'end' in finding:
                loc = (finding['start'].get('line', 0), finding['start'].get('col', 0))
                semgrep_locations.add(loc)
        
        for finding in cr_findings:
            if 'location' in finding:
                loc = (finding['location'].get('start_line', 0), finding['location'].get('start_column', 0))
                cr_locations.add(loc)
        
        missing_in_cr = semgrep_locations - cr_locations
        extra_in_cr = cr_locations - semgrep_locations
        
        if missing_in_cr:
            differences.append(f"CR缺失的位置: {missing_in_cr}")
        if extra_in_cr:
            differences.append(f"CR额外的位置: {extra_in_cr}")
        
        is_consistent = len(differences) == 0
        return is_consistent, differences
    
    def run_single_test(self, test_case: TestCase) -> TestResult:
        """运行单个测试用例"""
        print(f"运行测试: {test_case.directory}/{Path(test_case.rule_file).name} -> {Path(test_case.target_file).name}")

        # 运行semgrep
        semgrep_output, semgrep_error, semgrep_time = self.run_semgrep(
            test_case.rule_file, test_case.target_file
        )

        # 运行astgrep
        cr_output, cr_error, cr_time = self.run_cr_semservice(
            test_case.rule_file, test_case.target_file
        )

        # 调试信息
        if cr_error:
            print(f"  CR错误: {cr_error[:100]}...")
        if cr_output and len(cr_output) > 0:
            print(f"  CR输出长度: {len(cr_output)}")
            # 查找JSON部分
            if '"findings"' in cr_output:
                print(f"  CR输出包含findings")
                # 显示最后几行
                lines = cr_output.strip().split('\n')
                print(f"  最后3行:")
                for line in lines[-3:]:
                    print(f"    {line[:100]}")
            else:
                print(f"  CR输出不包含findings")

        # 解析结果
        semgrep_findings = self.parse_semgrep_output(semgrep_output) if semgrep_output else []
        cr_findings = self.parse_cr_semservice_output(cr_output) if cr_output else []

        print(f"  Semgrep发现: {len(semgrep_findings)}, CR发现: {len(cr_findings)}")

        # 比较结果
        is_consistent, differences = self.compare_results(semgrep_findings, cr_findings)

        return TestResult(
            test_case=test_case,
            semgrep_output=semgrep_output,
            cr_semservice_output=cr_output,
            semgrep_findings=semgrep_findings,
            cr_semservice_findings=cr_findings,
            semgrep_error=semgrep_error,
            cr_semservice_error=cr_error,
            execution_time_semgrep=semgrep_time,
            execution_time_cr=cr_time,
            is_consistent=is_consistent,
            differences=differences
        )
    
    def run_tests(self, max_cases: int = 10) -> List[TestResult]:
        """运行测试套件"""
        test_cases = self.discover_test_cases(max_cases)
        results = []
        
        for i, test_case in enumerate(test_cases, 1):
            print(f"\n[{i}/{len(test_cases)}] ", end="")
            result = self.run_single_test(test_case)
            results.append(result)
            
            # 显示结果摘要
            status = "✅ 一致" if result.is_consistent else "❌ 不一致"
            print(f"  {status}")
            if not result.is_consistent:
                for diff in result.differences[:2]:  # 只显示前2个差异
                    print(f"    - {diff}")
        
        self.results = results
        return results
    
    def generate_report(self) -> str:
        """生成测试报告"""
        if not self.results:
            return "没有测试结果"
        
        total = len(self.results)
        consistent = sum(1 for r in self.results if r.is_consistent)
        inconsistent = total - consistent
        
        report = f"""
测试报告
========

总测试用例: {total}
一致: {consistent} ({consistent/total*100:.1f}%)
不一致: {inconsistent} ({inconsistent/total*100:.1f}%)

不一致的测试用例:
"""
        
        for result in self.results:
            if not result.is_consistent:
                report += f"\n📁 {result.test_case.directory}\n"
                report += f"   规则: {Path(result.test_case.rule_file).name}\n"
                report += f"   目标: {Path(result.test_case.target_file).name}\n"
                for diff in result.differences:
                    report += f"   - {diff}\n"
        
        return report

if __name__ == "__main__":
    runner = TestRunner()
    results = runner.run_tests(max_cases=10)  # 现在测试10个用例
    print(runner.generate_report())
