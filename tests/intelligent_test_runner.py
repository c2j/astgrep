#!/usr/bin/env python3
"""
智能测试运行器 - 逐个比较semgrep和CR-SemService的结果，并在不一致时提供详细分析
"""

import os
import json
import subprocess
import sys
from pathlib import Path
from typing import Dict, List, Tuple, Set
import re

def run_semgrep(config_file: str, target_file: str) -> Tuple[List[Dict], str]:
    """运行semgrep并返回结果"""
    try:
        cmd = ["semgrep", "--config", config_file, target_file, "--json"]
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        
        if result.returncode != 0:
            print(f"❌ Semgrep失败: {result.stderr}")
            return [], result.stderr
            
        data = json.loads(result.stdout)
        findings = data.get("results", [])
        return findings, ""
        
    except Exception as e:
        print(f"❌ Semgrep执行错误: {e}")
        return [], str(e)

def run_cr_semservice(config_file: str, target_file: str) -> Tuple[List[Dict], str]:
    """运行CR-SemService并返回结果"""
    try:
        cmd = ["cargo", "run", "--bin", "cr-semservice", "analyze", 
               "--config", config_file, target_file]
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
        
        if result.returncode != 0:
            print(f"❌ CR-SemService失败: {result.stderr}")
            return [], result.stderr
            
        # 提取JSON部分 - 查找完整的JSON对象
        output = result.stdout.strip()

        # 查找JSON开始位置
        json_start = output.find('{\n  "findings"')
        if json_start == -1:
            json_start = output.find('{"findings"')
        if json_start == -1:
            json_start = output.find('{')

        if json_start == -1:
            print(f"❌ 无法找到JSON输出")
            return [], "No JSON output found"

        # 从JSON开始位置提取到文件末尾，但只取第一个完整的JSON对象
        json_part = output[json_start:]

        # 查找JSON结束位置（匹配大括号）
        brace_count = 0
        json_end = -1
        for i, char in enumerate(json_part):
            if char == '{':
                brace_count += 1
            elif char == '}':
                brace_count -= 1
                if brace_count == 0:
                    json_end = i + 1
                    break

        if json_end == -1:
            json_str = json_part
        else:
            json_str = json_part[:json_end]

        data = json.loads(json_str)
        findings = data.get("findings", [])
        return findings, ""
        
    except Exception as e:
        print(f"❌ CR-SemService执行错误: {e}")
        return [], str(e)

def extract_location(finding: Dict) -> Tuple[int, int]:
    """从发现中提取位置信息"""
    if "start" in finding:  # Semgrep格式
        return (finding["start"]["line"], finding["start"]["col"])
    elif "location" in finding:  # CR-SemService格式
        loc = finding["location"]
        return (loc["start_line"], loc["start_column"])
    else:
        return (0, 0)

def normalize_findings(findings: List[Dict]) -> Set[Tuple[int, int]]:
    """标准化发现为位置集合"""
    locations = set()
    for finding in findings:
        line, col = extract_location(finding)
        locations.add((line, col))
    return locations

def analyze_differences(semgrep_findings: List[Dict], cr_findings: List[Dict], 
                       target_file: str) -> Dict:
    """分析两个工具结果的差异"""
    semgrep_locations = normalize_findings(semgrep_findings)
    cr_locations = normalize_findings(cr_findings)
    
    missing_in_cr = semgrep_locations - cr_locations
    extra_in_cr = cr_locations - semgrep_locations
    
    analysis = {
        "semgrep_count": len(semgrep_findings),
        "cr_count": len(cr_findings),
        "missing_in_cr": missing_in_cr,
        "extra_in_cr": extra_in_cr,
        "is_consistent": len(missing_in_cr) == 0 and len(extra_in_cr) == 0
    }
    
    return analysis

def print_detailed_analysis(analysis: Dict, target_file: str, config_file: str):
    """打印详细的分析结果"""
    print(f"\n📁 测试文件: {target_file}")
    print(f"📋 配置文件: {config_file}")
    print(f"🔍 Semgrep发现: {analysis['semgrep_count']}")
    print(f"🔍 CR-SemService发现: {analysis['cr_count']}")
    
    if analysis["is_consistent"]:
        print("✅ 结果一致!")
        return True
    else:
        print("❌ 结果不一致!")
        
        if analysis["missing_in_cr"]:
            print(f"🔴 CR缺失的发现 ({len(analysis['missing_in_cr'])}个):")
            for line, col in sorted(analysis["missing_in_cr"]):
                print(f"   - 位置 ({line}, {col})")
                
        if analysis["extra_in_cr"]:
            print(f"🟡 CR额外的发现 ({len(analysis['extra_in_cr'])}个):")
            for line, col in sorted(analysis["extra_in_cr"]):
                print(f"   - 位置 ({line}, {col})")
        
        return False

def find_test_cases() -> List[Tuple[str, str]]:
    """查找所有测试用例"""
    test_cases = []
    test_dir = Path("tests/taint_maturity")
    
    for lang_dir in test_dir.iterdir():
        if lang_dir.is_dir():
            for yaml_file in lang_dir.glob("*.yaml"):
                # 查找对应的源文件
                base_name = yaml_file.stem
                for ext in [".py", ".java", ".js", ".sql", ".sh", ".php", ".cs", ".c"]:
                    source_file = lang_dir / f"{base_name}{ext}"
                    if source_file.exists():
                        test_cases.append((str(yaml_file), str(source_file)))
                        break
    
    return sorted(test_cases)

def main():
    """主函数"""
    print("🚀 智能测试运行器 - 逐个比较semgrep和CR-SemService")
    print("=" * 60)

    try:
        test_cases = find_test_cases()
        print(f"发现 {len(test_cases)} 个测试用例\n")

        if not test_cases:
            print("❌ 没有找到测试用例！")
            return

    except Exception as e:
        print(f"❌ 查找测试用例时出错: {e}")
        return
    
    consistent_count = 0
    total_count = len(test_cases)
    inconsistent_cases = []
    
    for i, (config_file, target_file) in enumerate(test_cases, 1):
        print(f"[{i}/{total_count}] 运行测试: {config_file} -> {Path(target_file).name}")
        
        # 运行semgrep
        semgrep_findings, semgrep_error = run_semgrep(config_file, target_file)
        if semgrep_error:
            print(f"⚠️  Semgrep错误，跳过: {semgrep_error}")
            continue
            
        # 运行CR-SemService
        cr_findings, cr_error = run_cr_semservice(config_file, target_file)
        if cr_error:
            print(f"⚠️  CR-SemService错误，跳过: {cr_error}")
            continue
            
        # 分析差异
        analysis = analyze_differences(semgrep_findings, cr_findings, target_file)
        is_consistent = print_detailed_analysis(analysis, target_file, config_file)
        
        if is_consistent:
            consistent_count += 1
        else:
            inconsistent_cases.append({
                "config": config_file,
                "target": target_file,
                "analysis": analysis
            })
            
        print("-" * 60)
    
    # 打印总结
    print(f"\n📊 测试总结")
    print("=" * 40)
    print(f"总测试用例: {total_count}")
    print(f"一致: {consistent_count} ({consistent_count/total_count*100:.1f}%)")
    print(f"不一致: {total_count-consistent_count} ({(total_count-consistent_count)/total_count*100:.1f}%)")
    
    if inconsistent_cases:
        print(f"\n🔧 需要修复的测试用例:")
        for case in inconsistent_cases:
            print(f"\n📁 {case['target']}")
            print(f"📋 {case['config']}")
            analysis = case['analysis']
            if analysis['missing_in_cr']:
                print(f"   🔴 缺失: {sorted(analysis['missing_in_cr'])}")
            if analysis['extra_in_cr']:
                print(f"   🟡 额外: {sorted(analysis['extra_in_cr'])}")

if __name__ == "__main__":
    main()
