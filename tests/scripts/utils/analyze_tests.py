#!/usr/bin/env python3
"""
分析tests目录结构，统计测试用例分布
"""

import os
import json
from pathlib import Path
from collections import defaultdict

def analyze_test_directory(tests_dir="tests"):
    """分析测试目录结构"""
    
    test_stats = {
        "total_directories": 0,
        "directories_with_yaml": 0,
        "total_yaml_files": 0,
        "total_target_files": 0,
        "language_distribution": defaultdict(int),
        "test_type_distribution": defaultdict(int),
        "directory_details": {}
    }
    
    # 支持的语言扩展名
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
        '.swift': 'swift',
        '.html': 'html',
        '.xml': 'xml',
        '.json': 'json',
        '.yaml': 'yaml',
        '.yml': 'yaml',
        '.dockerfile': 'dockerfile',
        '.tf': 'terraform'
    }
    
    def scan_directory(dir_path, relative_path=""):
        """递归扫描目录"""
        yaml_files = []
        target_files = []
        subdirs = []
        
        try:
            for item in os.listdir(dir_path):
                item_path = os.path.join(dir_path, item)
                
                if os.path.isdir(item_path):
                    subdirs.append(item)
                elif os.path.isfile(item_path):
                    ext = Path(item).suffix.lower()
                    if ext in ['.yaml', '.yml']:
                        yaml_files.append(item)
                    elif ext in language_extensions:
                        target_files.append(item)
                        test_stats["language_distribution"][language_extensions[ext]] += 1
        except PermissionError:
            pass
            
        return yaml_files, target_files, subdirs
    
    # 扫描tests目录
    tests_path = Path(tests_dir)
    if not tests_path.exists():
        print(f"错误：{tests_dir} 目录不存在")
        return test_stats
    
    for root, dirs, files in os.walk(tests_path):
        relative_root = os.path.relpath(root, tests_path)
        test_stats["total_directories"] += 1
        
        yaml_files = [f for f in files if f.endswith(('.yaml', '.yml'))]
        target_files = [f for f in files if Path(f).suffix.lower() in language_extensions]
        
        if yaml_files:
            test_stats["directories_with_yaml"] += 1
            test_stats["total_yaml_files"] += len(yaml_files)
        
        test_stats["total_target_files"] += len(target_files)
        
        # 分析测试类型
        dir_name = os.path.basename(root)
        if 'taint' in dir_name.lower():
            test_stats["test_type_distribution"]["taint_analysis"] += 1
        elif 'pattern' in dir_name.lower():
            test_stats["test_type_distribution"]["pattern_matching"] += 1
        elif 'parsing' in dir_name.lower():
            test_stats["test_type_distribution"]["parsing"] += 1
        elif 'autofix' in dir_name.lower():
            test_stats["test_type_distribution"]["autofix"] += 1
        elif 'rule' in dir_name.lower():
            test_stats["test_type_distribution"]["rules"] += 1
        else:
            test_stats["test_type_distribution"]["other"] += 1
        
        # 记录目录详情
        if yaml_files or target_files:
            test_stats["directory_details"][relative_root] = {
                "yaml_files": yaml_files,
                "target_files": target_files,
                "yaml_count": len(yaml_files),
                "target_count": len(target_files)
            }
    
    return test_stats

def print_analysis_report(stats):
    """打印分析报告"""
    print("=" * 60)
    print("TESTS目录分析报告")
    print("=" * 60)
    
    print(f"\n📊 总体统计:")
    print(f"  总目录数: {stats['total_directories']}")
    print(f"  包含YAML文件的目录数: {stats['directories_with_yaml']}")
    print(f"  总YAML文件数: {stats['total_yaml_files']}")
    print(f"  总目标文件数: {stats['total_target_files']}")
    
    print(f"\n🌍 语言分布:")
    for lang, count in sorted(stats['language_distribution'].items(), key=lambda x: x[1], reverse=True):
        print(f"  {lang}: {count} 文件")
    
    print(f"\n🔍 测试类型分布:")
    for test_type, count in sorted(stats['test_type_distribution'].items(), key=lambda x: x[1], reverse=True):
        print(f"  {test_type}: {count} 目录")
    
    print(f"\n📁 重要目录详情 (前20个):")
    sorted_dirs = sorted(stats['directory_details'].items(), 
                        key=lambda x: x[1]['yaml_count'] + x[1]['target_count'], 
                        reverse=True)
    
    for i, (dir_path, details) in enumerate(sorted_dirs[:20]):
        print(f"  {i+1:2d}. {dir_path}")
        print(f"      YAML: {details['yaml_count']}, 目标文件: {details['target_count']}")

if __name__ == "__main__":
    stats = analyze_test_directory()
    print_analysis_report(stats)
    
    # 保存详细结果到JSON文件
    with open("test_analysis_results.json", "w", encoding="utf-8") as f:
        # 转换defaultdict为普通dict以便JSON序列化
        stats_for_json = {
            "total_directories": stats["total_directories"],
            "directories_with_yaml": stats["directories_with_yaml"], 
            "total_yaml_files": stats["total_yaml_files"],
            "total_target_files": stats["total_target_files"],
            "language_distribution": dict(stats["language_distribution"]),
            "test_type_distribution": dict(stats["test_type_distribution"]),
            "directory_details": stats["directory_details"]
        }
        json.dump(stats_for_json, f, indent=2, ensure_ascii=False)
    
    print(f"\n💾 详细结果已保存到: test_analysis_results.json")
