# 配置文件语法检查功能实现方案

## 需求概述

为 xdotter 添加配置文件语法检查功能，支持：
1. **TOML 格式验证** - 使用内置 tomli 解析器
2. **JSON 格式验证** - 使用标准库 json 模块
3. **友好的错误提示** - 显示错误位置、原因和修复建议

---

## 功能设计

### 1. 支持的配置文件格式

| 格式 | 扩展名 | 优先级 | 说明 |
|------|--------|--------|------|
| TOML | `.toml` | 1 | 默认推荐格式 |
| JSON | `.json` | 2 | 备选格式 |

### 2. 配置文件查找顺序

```
1. 命令行指定的文件（如果未来支持 -c 参数）
2. xdotter.toml（当前目录）
3. xdotter.json（当前目录）
4. 错误：未找到配置文件
```

### 3. 命令设计

#### 3.1 独立验证命令

```bash
# 验证默认配置文件
xd validate

# 验证指定文件
xd validate myconfig.toml

# 验证多个文件
xd validate config1.toml config2.json

# 详细输出
xd validate -v

# 严格模式（警告也视为错误）
xd validate --strict
```

#### 3.2 部署时自动验证

```bash
# 部署时自动验证（默认行为）
xd deploy

# 跳过验证（紧急情况下）
xd deploy --no-validate
```

---

## 技术实现

### 1. 配置文件格式检测

```python
def detect_config_format(filepath: Path) -> Optional[str]:
    """
    检测配置文件格式
    
    Returns:
        'toml', 'json', or None if unknown
    """
    suffix = filepath.suffix.lower()
    if suffix == '.toml':
        return 'toml'
    elif suffix == '.json':
        return 'json'
    return None
```

### 2. TOML 验证

```python
def validate_toml(filepath: Path) -> Tuple[bool, str]:
    """
    验证 TOML 文件语法
    
    Returns:
        (is_valid, error_message)
    """
    try:
        content = filepath.read_text(encoding='utf-8')
        loads(content)  # tomli 解析
        return True, "TOML syntax is valid"
    except Exception as e:
        # tomli 会抛出 TomlDecodeError
        error_msg = format_toml_error(filepath, content, e)
        return False, error_msg
```

### 3. JSON 验证

```python
def validate_json(filepath: Path) -> Tuple[bool, str]:
    """
    验证 JSON 文件语法
    
    Returns:
        (is_valid, error_message)
    """
    try:
        content = filepath.read_text(encoding='utf-8')
        json.loads(content)
        return True, "JSON syntax is valid"
    except json.JSONDecodeError as e:
        error_msg = format_json_error(filepath, content, e)
        return False, error_msg
```

### 4. 错误格式化

#### TOML 错误示例

```python
def format_toml_error(filepath: Path, content: str, error: Exception) -> str:
    """
    格式化 TOML 错误信息
    
    输出示例：
    ❌ TOML 语法错误
    
    文件：xdotter.toml
    错误：Invalid TOML syntax (line 5, column 10)
    
    第 5 行:
      4 | [links]
    > 5 | ".bashrc" = "~/.bashrc
        |          ^
    错误：字符串未闭合（缺少引号）
    """
    # 解析错误位置
    line = error.lineno if hasattr(error, 'lineno') else 1
    col = error.pos if hasattr(error, 'pos') else 1
    
    # 提取错误行及其上下文
    lines = content.splitlines()
    error_line = lines[line - 1] if line <= len(lines) else ""
    prev_line = lines[line - 2] if line > 1 else ""
    next_line = lines[line] if line < len(lines) else ""
    
    # 构建错误信息
    msg = [
        f"{COLOR_RED}❌ TOML 语法错误{COLOR_RESET}",
        f"",
        f"文件：{filepath}",
        f"错误：{error.msg} (第 {line} 行，第 {col} 列)",
        f"",
        f"第 {line} 行:",
    ]
    
    if prev_line:
        msg.append(f"  {line-1} | {prev_line}")
    msg.append(f"{COLOR_RED}> {line} | {error_line}{COLOR_RESET}")
    msg.append(f"    | {' ' * (col-1)}^")
    if next_line:
        msg.append(f"  {line+1} | {next_line}")
    
    # 添加修复建议
    suggestion = get_toml_suggestion(error)
    if suggestion:
        msg.append(f"")
        msg.append(f"{COLOR_YELLOW}提示：{suggestion}{COLOR_RESET}")
    
    return "\n".join(msg)
```

#### JSON 错误示例

```python
def format_json_error(filepath: Path, content: str, error: json.JSONDecodeError) -> str:
    """
    格式化 JSON 错误信息
    
    输出示例：
    ❌ JSON 语法错误
    
    文件：config.json
    错误：Expecting ',' delimiter (line 3, column 15)
    
    第 3 行:
      2 |   "links": {
    > 3 |     ".bashrc": "~/.bashrc"
        |               ^
      4 |   }
    
    提示：JSON 对象属性之间需要用逗号分隔
    """
    line = error.lineno
    col = error.colno
    
    lines = content.splitlines()
    error_line = lines[line - 1] if line <= len(lines) else ""
    prev_line = lines[line - 2] if line > 1 else ""
    next_line = lines[line] if line < len(lines) else ""
    
    msg = [
        f"{COLOR_RED}❌ JSON 语法错误{COLOR_RESET}",
        f"",
        f"文件：{filepath}",
        f"错误：{error.msg} (第 {line} 行，第 {col} 列)",
        f"",
        f"第 {line} 行:",
    ]
    
    if prev_line:
        msg.append(f"  {line-1} | {prev_line}")
    msg.append(f"{COLOR_RED}> {line} | {error_line}{COLOR_RESET}")
    msg.append(f"    | {' ' * (col-1)}^")
    if next_line:
        msg.append(f"  {line+1} | {next_line}")
    
    # 添加修复建议
    suggestion = get_json_suggestion(error)
    if suggestion:
        msg.append(f"")
        msg.append(f"{COLOR_YELLOW}提示：{suggestion}{COLOR_RESET}")
    
    return "\n".join(msg)
```

### 5. 常见错误及建议

```python
TOML_SUGGESTIONS = {
    "Invalid initial character for a key": "TOML 键名不能以特殊字符开头，请用引号包裹",
    "Expected '=' after a key": "TOML 键值对需要使用 = 连接",
    "Unclosed string": "字符串未闭合，请检查引号是否配对",
    "Invalid number": "数字格式错误，检查是否有前导零或非法字符",
    "Invalid value": "无效的值，TOML 支持：字符串、数字、布尔值、日期、数组、表格",
    "Key appears more than once": "键名重复，TOML 不允许重复键名",
    "Unquoted string": "字符串必须用引号包裹（双引号或单引号）",
}

JSON_SUGGESTIONS = {
    "Expecting ',' delimiter": "JSON 对象属性之间需要用逗号分隔",
    "Expecting property name": "JSON 键名必须是字符串（用双引号包裹）",
    "Expecting ':' delimiter": "JSON 键值对需要使用冒号分隔",
    "Expecting value": "JSON 值必须是：字符串、数字、布尔值、null、数组或对象",
    "Unterminated string": "字符串未闭合，检查引号是否配对",
    "Invalid control character": "JSON 不支持控制字符，使用转义序列（如 \\n）",
    "Extra data": "JSON 文件只能包含一个顶层值（对象或数组）",
}

def get_toml_suggestion(error: Exception) -> Optional[str]:
    """根据错误类型返回修复建议"""
    error_msg = str(error).lower()
    for key, suggestion in TOML_SUGGESTIONS.items():
        if key.lower() in error_msg:
            return suggestion
    return None

def get_json_suggestion(error: json.JSONDecodeError) -> Optional[str]:
    """根据错误类型返回修复建议"""
    error_msg = error.msg.lower()
    for key, suggestion in JSON_SUGGESTIONS.items():
        if key.lower() in error_msg:
            return suggestion
    return None
```

---

## 命令实现

### validate 命令

```python
def cmd_validate(args) -> int:
    """
    验证配置文件语法
    
    Returns:
        0 if all files are valid, 1 otherwise
    """
    files_to_check = args.files if args.files else ['xdotter.toml', 'xdotter.json']
    
    all_valid = True
    results = []
    
    for filepath_str in files_to_check:
        filepath = Path(filepath_str)
        
        if not filepath.exists():
            if filepath_str in ['xdotter.toml', 'xdotter.json']:
                # 默认文件不存在，跳过
                continue
            else:
                log(args, "error", f"File not found: {filepath}")
                all_valid = False
                continue
        
        # 检测格式
        fmt = detect_config_format(filepath)
        if fmt is None:
            log(args, "error", f"Unknown file format: {filepath.suffix}")
            all_valid = False
            continue
        
        # 验证
        if fmt == 'toml':
            is_valid, msg = validate_toml(filepath)
        else:  # json
            is_valid, msg = validate_json(filepath)
        
        if is_valid:
            log(args, "info", f"{COLOR_GREEN}✓{COLOR_RESET} {filepath} ({fmt.upper()})")
            results.append((filepath, True, fmt))
        else:
            log(args, "error", msg)
            all_valid = False
            results.append((filepath, False, fmt))
    
    # 摘要
    if not args.quiet:
        total = len(results)
        valid = sum(1 for _, v, _ in results if v)
        invalid = total - valid
        
        log(args, "info", "")
        if invalid == 0:
            log(args, "info", f"{COLOR_GREEN}✓ 所有 {total} 个配置文件语法正确{COLOR_RESET}")
        else:
            log(args, "warning", f"✗ {invalid}/{total} 个配置文件存在语法错误{COLOR_RESET}")
    
    return 0 if all_valid else 1
```

### 部署时自动验证

```python
def deploy_on(config_file: str, args) -> bool:
    """Deploy dotfiles from a config file"""
    
    # 1. 验证配置语法（除非跳过）
    if not getattr(args, 'no_validate', False):
        filepath = Path(config_file)
        fmt = detect_config_format(filepath)
        
        if fmt == 'toml':
            is_valid, msg = validate_toml(filepath)
        elif fmt == 'json':
            is_valid, msg = validate_json(filepath)
        else:
            log(args, "error", f"Unsupported config format: {filepath.suffix}")
            return False
        
        if not is_valid:
            log(args, "error", msg)
            log(args, "error", "Deployment aborted due to config syntax errors")
            log(args, "info", "Hint: Run 'xd validate' to check config syntax")
            return False
    
    # 2. 继续部署逻辑...
    # ... existing code ...
```

---

## 命令行参数

### 新增参数

```python
parser.add_argument(
    "command",
    nargs="?",
    choices=["deploy", "undeploy", "check-permissions", "validate", "new", "help", "version"],
    help="Command to execute",
)

parser.add_argument(
    "--no-validate",
    action="store_true",
    help="Skip config syntax validation during deploy",
)

parser.add_argument(
    "--strict",
    action="store_true",
    help="Treat warnings as errors during validation",
)
```

---

## 使用示例

### 场景 1：CI/CD 验证

```yaml
# .github/workflows/ci.yml
- name: Validate config syntax
  run: python xd.py validate
```

### 场景 2：部署前检查

```bash
# 检查配置
xd validate

# 如果通过，部署
xd deploy
```

### 场景 3：调试配置错误

```bash
# 详细输出
xd validate -v

# 输出：
# ❌ TOML 语法错误
# 文件：xdotter.toml
# 第 5 行:
#   4 | [links]
# > 5 | ".bashrc" = "~/.bashrc
#     |          ^
# 错误：字符串未闭合
# 提示：TOML 字符串必须用引号包裹
```

---

## 测试计划

### 单元测试

```python
def test_validate_valid_toml():
    """Test validating a correct TOML file"""
    # ...

def test_validate_invalid_toml():
    """Test validating an incorrect TOML file"""
    # ...

def test_validate_valid_json():
    """Test validating a correct JSON file"""
    # ...

def test_validate_invalid_json():
    """Test validating an incorrect JSON file"""
    # ...

def test_deploy_with_invalid_config():
    """Test that deploy fails with invalid config"""
    # ...

def test_deploy_skip_validate():
    """Test deploy with --no-validate flag"""
    # ...
```

### 集成测试

```bash
# 创建有效配置
echo '[links]
".bashrc" = "~/.bashrc"' > xdotter.toml
xd validate  # 应该通过

# 创建无效配置
echo '[links
".bashrc" = "~/.bashrc"' > xdotter.toml
xd validate  # 应该失败
```

---

## 实现优先级

1. **Phase 1** - TOML 验证（核心功能）
2. **Phase 2** - JSON 验证（可选功能）
3. **Phase 3** - 部署时自动验证
4. **Phase 4** - 详细错误提示和建议

---

## 代码量估算

| 模块 | 行数 | 复杂度 |
|------|------|--------|
| 格式检测 | ~20 | 低 |
| TOML 验证 | ~30 | 低 |
| JSON 验证 | ~30 | 低 |
| 错误格式化 | ~80 | 中 |
| validate 命令 | ~60 | 中 |
| 部署集成 | ~30 | 低 |
| 测试用例 | ~100 | 中 |
| **总计** | **~350** | **中** |

---

## 依赖

- **TOML**: 使用现有 `tomli`（已 vendored）
- **JSON**: Python 标准库 `json`
- **无额外依赖**

---

## 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 解析错误信息不完整 | 用户体验差 | 捕获异常并提取尽可能多的信息 |
| 验证增加部署时间 | 用户不满 | 提供 `--no-validate` 跳过 |
| 误报（有效配置报错误） | 用户困惑 | 充分测试，提供反馈渠道 |

---

## 结论

**推荐实现方案 C**：
- 独立的 `validate` 命令
- 部署时自动验证（可跳过）
- 支持 TOML 和 JSON
- 友好的错误提示和修复建议

这个方案平衡了功能完整性、用户体验和实现复杂度。
