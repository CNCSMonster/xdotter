# Implementation Plan: 修复 SPEC gap 错误检测的三个问题

## Overview
修复 `detect_link_nesting` 实现中的路径前缀匹配缺陷、清理占位测试、更新过时文档。

## Architecture Decisions
- **路径分隔符检查：** 直接在 `src/plan.rs` 中修复，不在 `path.rs` 中抽取独立函数（避免过度抽象，仅此一处需要）
- **`std::path::Component` 解析：** 用 Rust 标准库的 `Path::components()` 逐组件比较，而非字符串截断，天然正确处理 `/` 边界
- **占位测试：** 删除 `test_symlink_safety.py` 中的 no-op，因为 `test_critical_high_gaps.py` 已有有效版本
- **docstring：** 更新为描述当前（修复后）行为

## Task List

### Phase 1: 代码修复
- [x] Task 1: 修复 `detect_link_nesting` 路径分隔符边界检查

### Checkpoint: 编译 + 全部测试
- [x] `cargo build` 成功
- [x] `cargo test --bin xd`（35 单元测试）全部通过
- [x] `cargo test --test integration`（21 集成测试）全部通过
- [x] `pytest tests/spec/`（167 SPEC 测试）全部通过

### Phase 2: 测试文档清理
- [x] Task 2: 删除 `test_symlink_safety.py` 中的占位测试
- [x] Task 3: 更新 `test_critical_high_gaps.py` 中的过时 docstring

### Checkpoint: 最终验证
- [x] 全部测试通过（223 passed）

## Risks and Mitigations
| Risk | Impact | Mitigation |
|------|--------|------------|
| 路径边界检查算法错误导致漏检 | Med | 验证现有测试 `test_link_inside_source_rejected` 仍通过 |
| 删除测试导致覆盖率下降 | Low | `test_critical_high_gaps.py` 中有完整替代 |

## Open Questions
无。三个问题边界清晰，无歧义。
