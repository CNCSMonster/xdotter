# 命令行自动补全功能实现方案

## 需求分析

### 当前问题
```bash
# 用户输入 xd 后按 Tab，没有任何提示
xd <TAB>  # 无反应

# 用户需要记住所有命令和参数
xd --help  # 必须查看帮助才能知道可用选项
```

### 目标体验
```bash
# 输入 xd 后按 Tab，显示所有命令
xd <TAB>
deploy  undeploy  validate  check-permissions  new  help  version

# 输入部分命令后按 Tab，自动补全
xd de<TAB>  →  xd deploy

# 输入命令后按 Tab，显示参数
xd deploy <TAB>
-v  -q  -n  -i  -f  --verbose  --dry-run  --check-permissions ...

# 输入参数后按 Tab，显示值
xd --config <TAB>
xdotter.toml  xdotter.json  (当前目录下的配置文件)
```

---

## 实现方案对比

### 方案 A：Shell 补全脚本（推荐）

为不同 Shell 生成补全脚本，用户 source 后即可使用。

**支持 Shell：**
- Bash (4.2+)
- Zsh (5.0+)
- Fish (3.0+)

**优点：**
- ✅ 标准做法，符合 Unix 传统
- ✅ 性能好（Shell 原生处理）
- ✅ 无需修改主程序逻辑
- ✅ 易于分发和维护

**缺点：**
- ❌ 需要用户手动安装补全脚本
- ❌ 不同 Shell 需要不同脚本

**代表项目：**
- `kubectl completion bash`
- `git completion`
- `cargo completion`

---

### 方案 B：Python 补全库

使用 Python 库（如 `argcomplete`）实现补全。

**优点：**
- ✅ 代码量少
- ✅ 自动从 argparse 生成

**缺点：**
- ❌ 需要额外依赖
- ❌ 性能较慢（每次 Tab 都要启动 Python）
- ❌ 需要修改 shebang 或使用特殊包装

**代表项目：**
- `argcomplete`
- `click.completion`

---

### 方案 C：混合方案（最佳）

- 默认提供 Shell 补全脚本（方案 A）
- 可选生成补全脚本命令（`xd completion bash`）
- 未来可考虑 argcomplete（可选依赖）

---

## 详细实现方案（方案 C）

### 1. 命令设计

```bash
# 生成补全脚本
xd completion bash      # 输出 Bash 补全脚本
xd completion zsh       # 输出 Zsh 补全脚本
xd completion fish      # 输出 Fish 补全脚本

# 安装补全脚本（可选）
xd completion bash --install
xd completion zsh --install
```

### 2. 补全内容

#### Bash 补全示例

```bash
# ~/.local/share/bash-completion/completions/xd

_xd_completions() {
    local cur prev words cword
    _init_completion -n := || return

    # 命令补全
    if [[ $cword -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "deploy undeploy validate check-permissions new help version" -- "$cur") )
        return
    fi

    # 参数补全
    case "${words[1]}" in
        deploy|undeploy)
            COMPREPLY=( $(compgen -W "-v -q -n -i -f --verbose --quiet --dry-run --interactive --force --check-permissions --fix-permissions --no-validate" -- "$cur") )
            ;;
        validate)
            COMPREPLY=( $(compgen -W "-v -q -n --verbose --quiet --dry-run" -- "$cur") )
            ;;
        check-permissions)
            COMPREPLY=( $(compgen -W "-v -q -n --verbose --quiet --dry-run --fix-permissions" -- "$cur") )
            ;;
        *)
            COMPREPLY=( $(compgen -W "-v -q -n -h --verbose --quiet --dry-run --help" -- "$cur") )
            ;;
    esac
}

complete -F _xd_completions xd
```

#### Zsh 补全示例

```zsh
# ~/.local/share/zsh/site-functions/_xd

#compdef xd

local context state line
typeset -A opt_args

_arguments \
    '1:command:(deploy undeploy validate check-permissions new help version)' \
    '(-v --verbose)*-v[Show more information]' \
    '(-v --verbose)*--verbose[Show more information]' \
    '(-q --quiet)*-q[Do not print any output]' \
    '(-q --quiet)*--quiet[Do not print any output]' \
    '(-n --dry-run)*-n[Show what would be done]' \
    '(-n --dry-run)*--dry-run[Show what would be done]' \
    '(-i --interactive)*-i[Ask for confirmation]' \
    '(-i --interactive)*--interactive[Ask for confirmation]' \
    '(-f --force)*-f[Force overwrite existing files]' \
    '(-f --force)*--force[Force overwrite existing files]' \
    '--check-permissions[Check permissions for sensitive files]' \
    '--fix-permissions[Fix permissions for sensitive files]' \
    '--no-validate[Skip config syntax validation]' \
    '-h[Print help message]' \
    '--help[Print help message]' \
    '-V[Print version]' \
    '--version[Print version]'
```

#### Fish 补全示例

```fish
# ~/.config/fish/completions/xd.fish

complete -c xd -n "__fish_use_subcommand" -a deploy -d "Deploy dotfiles"
complete -c xd -n "__fish_use_subcommand" -a undeploy -d "Remove deployed dotfiles"
complete -c xd -n "__fish_use_subcommand" -a validate -d "Validate configuration syntax"
complete -c xd -n "__fish_use_subcommand" -a check-permissions -d "Check file permissions"
complete -c xd -n "__fish_use_subcommand" -a new -d "Create new config template"
complete -c xd -n "__fish_use_subcommand" -a help -d "Print help message"
complete -c xd -n "__fish_use_subcommand" -a version -d "Print version"

complete -c xd -s v -l verbose -d "Show more information"
complete -c xd -s q -l quiet -d "Do not print any output"
complete -c xd -s n -l dry-run -d "Show what would be done"
complete -c xd -s i -l interactive -d "Ask for confirmation"
complete -c xd -s f -l force -d "Force overwrite"
complete -c xd -l check-permissions -d "Check permissions"
complete -c xd -l fix-permissions -d "Fix permissions"
complete -c xd -l no-validate -d "Skip validation"
```

---

### 3. 代码实现

#### 添加 completion 命令

```python
def cmd_completion(args) -> int:
    """
    Generate shell completion scripts.
    
    Usage:
        xd completion bash
        xd completion zsh
        xd completion fish
    """
    shell = args.shell.lower()
    
    if shell == 'bash':
        print(BASH_COMPLETION_SCRIPT)
        return 0
    elif shell == 'zsh':
        print(ZSH_COMPLETION_SCRIPT)
        return 0
    elif shell == 'fish':
        print(FISH_COMPLETION_SCRIPT)
        return 0
    else:
        log(args, "error", f"Unsupported shell: {shell}")
        log(args, "info", "Supported shells: bash, zsh, fish")
        return 1
```

#### 补全脚本模板

```python
BASH_COMPLETION_SCRIPT = '''# Bash completion for xdotter
# Place in: ~/.local/share/bash-completion/completions/xd
# Or source: source <(xd completion bash)

_xd_completions() {
    local cur prev words cword
    _init_completion -n := || return

    if [[ $cword -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "deploy undeploy validate check-permissions new help version" -- "$cur") )
        return
    fi

    case "${words[1]}" in
        deploy|undeploy)
            COMPREPLY=( $(compgen -W "-v -q -n -i -f --verbose --quiet --dry-run --interactive --force --check-permissions --fix-permissions --no-validate" -- "$cur") )
            ;;
        validate)
            COMPREPLY=( $(compgen -W "-v -q -n --verbose --quiet --dry-run" -- "$cur") )
            ;;
        check-permissions)
            COMPREPLY=( $(compgen -W "-v -q -n --verbose --quiet --dry-run --fix-permissions" -- "$cur") )
            ;;
        *)
            COMPREPLY=( $(compgen -W "-v -q -n -h --verbose --quiet --dry-run --help" -- "$cur") )
            ;;
    esac
}

complete -F _xd_completions xd
'''

ZSH_COMPLETION_SCRIPT = '''# Zsh completion for xdotter
# Place in: ~/.local/share/zsh/site-functions/_xd
# Or autoload: autoload -Uz compinit && compinit

#compdef xd

_arguments \\
    '1:command:(deploy undeploy validate check-permissions new help version)' \\
    '(-v --verbose)*-v[Show more information]' \\
    '(-v --verbose)*--verbose[Show more information]' \\
    '(-q --quiet)*-q[Do not print any output]' \\
    '(-q --quiet)*--quiet[Do not print any output]' \\
    '(-n --dry-run)*-n[Show what would be done]' \\
    '(-n --dry-run)*--dry-run[Show what would be done]' \\
    '(-i --interactive)*-i[Ask for confirmation]' \\
    '(-i --interactive)*--interactive[Ask for confirmation]' \\
    '(-f --force)*-f[Force overwrite existing files]' \\
    '(-f --force)*--force[Force overwrite existing files]' \\
    '--check-permissions[Check permissions for sensitive files]' \\
    '--fix-permissions[Fix permissions for sensitive files]' \\
    '--no-validate[Skip config syntax validation]' \\
    '-h[Print help message]' \\
    '--help[Print help message]' \\
    '-V[Print version]' \\
    '--version[Print version]'
'''

FISH_COMPLETION_SCRIPT = '''# Fish completion for xdotter
# Place in: ~/.config/fish/completions/xd.fish
# Or source: source (xd completion fish | psub)

complete -c xd -n "__fish_use_subcommand" -a deploy -d "Deploy dotfiles"
complete -c xd -n "__fish_use_subcommand" -a undeploy -d "Remove deployed dotfiles"
complete -c xd -n "__fish_use_subcommand" -a validate -d "Validate configuration syntax"
complete -c xd -n "__fish_use_subcommand" -a check-permissions -d "Check file permissions"
complete -c xd -n "__fish_use_subcommand" -a new -d "Create new config template"
complete -c xd -n "__fish_use_subcommand" -a help -d "Print help message"
complete -c xd -n "__fish_use_subcommand" -a version -d "Print version"

complete -c xd -s v -l verbose -d "Show more information"
complete -c xd -s q -l quiet -d "Do not print any output"
complete -c xd -s n -l dry-run -d "Show what would be done"
complete -c xd -s i -l interactive -d "Ask for confirmation"
complete -c xd -s f -l force -d "Force overwrite"
complete -c xd -l check-permissions -d "Check permissions"
complete -c xd -l fix-permissions -d "Fix permissions"
complete -c xd -l no-validate -d "Skip validation"
'''
```

---

### 4. 安装说明

#### Bash

```bash
# 方法 1：直接 source（临时）
source <(xd completion bash)

# 方法 2：保存到补全目录（永久）
xd completion bash > ~/.local/share/bash-completion/completions/xd

# 方法 3：添加到 ~/.bashrc（永久）
echo 'source <(xd completion bash)' >> ~/.bashrc
```

#### Zsh

```bash
# 方法 1：直接 source（临时）
source <(xd completion zsh)

# 方法 2：保存到补全目录（永久）
xd completion zsh > ~/.local/share/zsh/site-functions/_xd

# 方法 3：添加到 ~/.zshrc（永久）
echo 'source <(xd completion zsh)' >> ~/.zshrc
```

#### Fish

```bash
# 方法 1：直接 source（临时）
source (xd completion fish | psub)

# 方法 2：保存到补全目录（永久）
xd completion fish > ~/.config/fish/completions/xd.fish

# 方法 3：添加到 config.fish（永久）
echo 'source (xd completion fish | psub)' >> ~/.config/fish/config.fish
```

---

### 5. 高级功能（可选）

#### 5.1 动态参数补全

```bash
# 补全配置文件名
xd --config <TAB>
xdotter.toml  xdotter.json  (仅显示 .toml/.json 文件)

# 补全命令（validate 时）
xd validate <TAB>
config1.toml  config2.json  (当前目录下的配置文件)
```

实现：
```python
def _complete_config_files():
    """Complete config file names"""
    import glob
    files = glob.glob('*.toml') + glob.glob('*.json')
    return files
```

#### 5.2 智能命令补全

```bash
# 根据已输入的参数，智能提示剩余参数
xd deploy -v <TAB>
# 不再提示 -v/--verbose（已输入）
# 提示其他未使用的参数
```

#### 5.3 帮助信息补全

```bash
# 显示参数说明
xd deploy --<TAB>
--verbose          Show more information
--dry-run          Show what would be done
--check-permissions Check permissions
```

---

## 实现优先级

### Phase 1：基础补全（推荐先实现）
- ✅ `xd completion bash/zsh/fish` 命令
- ✅ 静态命令和参数补全
- ✅ 安装说明文档

### Phase 2：动态补全
- 🔲 配置文件名补全
- 🔲 智能参数过滤

### Phase 3：增强功能
- 🔲 帮助信息补全
- 🔲 自动安装脚本

---

## 代码量估算

| 模块 | 行数 | 复杂度 |
|------|------|--------|
| completion 命令 | ~50 | 低 |
| Bash 补全脚本 | ~40 | 低 |
| Zsh 补全脚本 | ~30 | 低 |
| Fish 补全脚本 | ~20 | 低 |
| 安装说明文档 | ~30 | 低 |
| **总计** | **~170** | **低** |

---

## 测试计划

### Bash 测试

```bash
# 1. 生成补全脚本
xd completion bash | head -20

# 2. 临时加载
source <(xd completion bash)

# 3. 测试补全
xd <TAB>        # 应显示所有命令
xd de<TAB>      # 应补全为 deploy
xd deploy <TAB> # 应显示所有参数
```

### Zsh 测试

```bash
# 1. 生成补全脚本
xd completion zsh | head -20

# 2. 临时加载
source <(xd completion zsh)

# 3. 测试补全
xd <TAB>        # 应显示所有命令
```

### Fish 测试

```bash
# 1. 生成补全脚本
xd completion fish | head -20

# 2. 临时加载
source (xd completion fish | psub)

# 3. 测试补全
xd <TAB>        # 应显示所有命令
```

---

## 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Shell 兼容性问题 | 部分用户无法使用 | 提供多种 Shell 支持 |
| 补全脚本错误 | 补全不准确 | 充分测试各 Shell |
| 安装复杂 | 用户不愿使用 | 提供一键安装脚本 |

---

## 结论

**推荐实现方案：Phase 1（基础补全）**

- 实现简单（~170 行代码）
- 用户体验提升明显
- 符合行业标准做法
- 易于维护和扩展

**实现步骤：**
1. 添加 `completion` 命令
2. 提供 Bash/Zsh/Fish 补全脚本
3. 更新 README 添加安装说明
4. 测试各 Shell 补全功能
