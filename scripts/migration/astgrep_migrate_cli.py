#!/usr/bin/env python3
"""
ASTGreP 迁移CLI工具
替代复杂的Rust CLI工具，提供实际的迁移功能
"""

import argparse
import json
import os
import shutil
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Tuple

class ASTGrePMigrator:
    def __init__(self, project_root: Path):
        self.project_root = project_root
        self.newtest_root = project_root / "newtest"
        self.testcases_root = self.newtest_root / "testcases"
        self.scripts_root = self.newtest_root / "scripts"

        # 语言映射
        self.language_mapping = {
            '.py': 'python',
            '.js': 'javascript',
            '.jsx': 'javascript',
            '.ts': 'typescript',
            '.tsx': 'typescript',
            '.java': 'java',
            '.rb': 'ruby',
            '.go': 'go',
            '.rs': 'rust',
            '.php': 'php',
            '.cs': 'csharp',
            '.cpp': 'cpp',
            '.c': 'c',
            '.cc': 'cpp',
            '.cxx': 'cpp',
            '.sh': 'bash',
            '.bash': 'bash',
            '.zsh': 'bash',
            '.sql': 'sql',
            '.xml': 'xml',
            '.html': 'html',
            '.json': 'json',
            '.kt': 'kotlin',
            '.swift': 'swift',
            '.pl': 'perl',
            '.pm': 'perl',
            '.r': 'r',
            '.R': 'r',
            '.scala': 'scala',
            '.hs': 'haskell',
            '.dart': 'dart',
            '.lua': 'lua',
            '.swift': 'swift',
        }

        # 测试类型分类
        self.test_type_keywords = {
            'security': ['security', 'injection', 'xss', 'csrf', 'auth', 'password', 'crypto'],
            'performance': ['performance', 'perf', 'benchmark', 'speed', 'optimize'],
            'integration': ['integration', 'end-to-end', 'e2e', 'workflow'],
            'parsing': ['parse', 'syntax', 'lexer', 'parser'],
            'compatibility': ['compatibility', 'version', 'cross-platform'],
            'pattern-matching': ['pattern', 'regex', 'match', 'search'],
            'rule-validation': ['rule', 'validate', 'check', 'verify'],
        }

    def detect_language(self, file_path: Path) -> str:
        """检测文件的语言"""
        ext = file_path.suffix.lower()
        return self.language_mapping.get(ext, 'misc')

    def classify_test_type(self, file_path: Path) -> str:
        """根据文件名和内容分类测试类型"""
        name_lower = file_path.name.lower()

        # 检查文件名中是否包含特定关键词
        for test_type, keywords in self.test_type_keywords.items():
            if any(keyword in name_lower for keyword in keywords):
                return test_type

        # 默认分类
        if 'test' in name_lower or 'spec' in name_lower:
            return 'basic'
        elif 'example' in name_lower:
            return 'pattern-matching'
        else:
            return 'basic'

    def discover_test_cases(self, source_dir: Path) -> List[Tuple[Path, str, str]]:
        """发现所有测试用例"""
        test_cases = []

        if not source_dir.exists():
            return test_cases

        for yaml_file in source_dir.rglob("*.yaml"):
            # 检测语言
            language = self.detect_language(yaml_file)

            # 分类测试类型
            test_type = self.classify_test_type(yaml_file)

            test_cases.append((yaml_file, language, test_type))

        return test_cases

    def get_target_path(self, source_file: Path, language: str, test_type: str) -> Path:
        """生成目标路径"""
        return self.testcases_root / language / test_type / source_file.name

    def migrate_file(self, source_file: Path, target_file: Path, dry_run: bool = False) -> bool:
        """迁移单个文件"""
        try:
            if dry_run:
                print(f"[DRY RUN] Would migrate: {source_file} -> {target_file}")
                return True

            # 确保目标目录存在
            target_file.parent.mkdir(parents=True, exist_ok=True)

            # 复制文件
            shutil.copy2(source_file, target_file)
            print(f"✅ Migrated: {source_file} -> {target_file}")
            return True

        except Exception as e:
            print(f"❌ Failed to migrate {source_file}: {e}")
            return False

    def migrate_script(self, source_file: Path, dry_run: bool = False) -> bool:
        """迁移脚本文件"""
        try:
            target_file = self.scripts_root / "validation" / source_file.name

            if dry_run:
                print(f"[DRY RUN] Would migrate script: {source_file} -> {target_file}")
                return True

            target_file.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source_file, target_file)
            print(f"✅ Migrated script: {source_file} -> {target_file}")
            return True

        except Exception as e:
            print(f"❌ Failed to migrate script {source_file}: {e}")
            return False

    def migrate_all(self, source_dir: Path, dry_run: bool = False) -> Dict:
        """迁移所有测试用例"""
        print(f"🚀 Starting migration from {source_dir}")

        # 发现测试用例
        test_cases = self.discover_test_cases(source_dir)
        print(f"📊 Found {len(test_cases)} test cases")

        # 统计数据
        stats = {
            'total_files': 0,
            'successful_migrations': 0,
            'failed_migrations': 0,
            'by_language': {},
            'by_test_type': {},
            'migrated_files': []
        }

        # 迁移测试用例
        for source_file, language, test_type in test_cases:
            stats['total_files'] += 1

            # 更新统计
            stats['by_language'][language] = stats['by_language'].get(language, 0) + 1
            stats['by_test_type'][test_type] = stats['by_test_type'].get(test_type, 0) + 1

            # 获取目标路径
            target_file = self.get_target_path(source_file, language, test_type)

            # 执行迁移
            if self.migrate_file(source_file, target_file, dry_run):
                stats['successful_migrations'] += 1
                stats['migrated_files'].append({
                    'source': str(source_file),
                    'target': str(target_file),
                    'language': language,
                    'test_type': test_type
                })
            else:
                stats['failed_migrations'] += 1

        # 迁移脚本文件
        script_files = list(source_dir.rglob("*.sh"))
        print(f"📁 Found {len(script_files)} script files")

        for script_file in script_files:
            if self.migrate_script(script_file, dry_run):
                stats['successful_migrations'] += 1
            else:
                stats['failed_migrations'] += 1

        stats['total_files'] += len(script_files)

        return stats

    def generate_report(self, stats: Dict, output_file: Optional[str] = None):
        """生成迁移报告"""
        report = {
            'migration_time': datetime.now().isoformat(),
            'summary': {
                'total_files': stats['total_files'],
                'successful_migrations': stats.get('successful_migrations', 0),
                'failed_migrations': stats.get('failed_migrations', 0),
                'success_rate': (stats.get('successful_migrations', 0) / stats['total_files'] * 100) if stats['total_files'] > 0 else 0
            },
            'by_language': stats['by_language'],
            'by_test_type': stats['by_test_type'],
            'migrated_files': stats.get('migrated_files', [])
        }

        # 打印报告
        print("\n" + "="*80)
        print("MIGRATION REPORT")
        print("="*80)
        print(f"Total files: {report['summary']['total_files']}")
        if 'successful_migrations' in report['summary']:
            print(f"Successful: {report['summary']['successful_migrations']}")
            print(f"Failed: {report['summary']['failed_migrations']}")
            print(f"Success rate: {report['summary']['success_rate']:.1f}%")
        else:
            print("Status: Analysis only")

        print("\nBy language:")
        for lang, count in report['by_language'].items():
            print(f"  {lang}: {count} files")

        print("\nBy test type:")
        for test_type, count in report['by_test_type'].items():
            print(f"  {test_type}: {count} files")

        print("="*80)

        # 保存报告到文件
        if output_file:
            with open(output_file, 'w') as f:
                json.dump(report, f, indent=2)
            print(f"\n📊 Detailed report saved to: {output_file}")

def main():
    parser = argparse.ArgumentParser(description='ASTGreP Migration CLI Tool')
    parser.add_argument('--project-root', type=Path, default=Path('.'),
                        help='Project root directory')
    parser.add_argument('--source-dir', type=Path, default=None,
                        help='Source directory to migrate from')
    parser.add_argument('--dry-run', action='store_true',
                        help='Show what would be migrated without actually doing it')
    parser.add_argument('--output', type=str,
                        help='Output report file')
    parser.add_argument('--report-only', action='store_true',
                        help='Only analyze and report, no migration')

    args = parser.parse_args()

    # 自动检测源目录
    if args.source_dir is None:
        if (args.project_root / "tests").exists():
            source_dir = args.project_root / "tests"
        else:
            print("❌ No tests directory found. Please specify --source-dir")
            sys.exit(1)
    else:
        source_dir = args.source_dir

    # 创建迁移器
    migrator = ASTGrePMigrator(args.project_root)

    if args.report_only:
        # 只分析不迁移
        print("📊 Analyzing source directory...")
        test_cases = migrator.discover_test_cases(source_dir)
        print(f"Found {len(test_cases)} test cases")

        stats = {
            'total_files': len(test_cases),
            'by_language': {},
            'by_test_type': {}
        }

        for _, language, test_type in test_cases:
            stats['by_language'][language] = stats['by_language'].get(language, 0) + 1
            stats['by_test_type'][test_type] = stats['by_test_type'].get(test_type, 0) + 1

        migrator.generate_report(stats, args.output)
        return

    # 执行迁移
    stats = migrator.migrate_all(source_dir, args.dry_run)

    # 生成报告
    migrator.generate_report(stats, args.output)

    # 退出状态
    if stats['failed_migrations'] > 0:
        sys.exit(1)
    else:
        sys.exit(0)

if __name__ == "__main__":
    main()