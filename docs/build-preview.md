# 只构建、不发布

现有 `release.yml` 支持手动构建并上传 Artifacts。请从待验证的分支运行，**`tag` 保持空字符串**；分支名也不能是版本标签。

```sh
gh workflow run release.yml --ref codex/media-library-transition \
  -f platforms=check -f quick=true -f tag=

# 编译检查与单测通过后，构建所有测试包：
gh workflow run release.yml --ref codex/media-library-transition \
  -f platforms=all -f quick=true -f tag=
```

`quick=true` 的 Android 仅构建 arm64，macOS 仅构建 Apple Silicon。Windows、Linux 和未签名 iOS 包按原工作流构建。Android 使用仓库已有签名配置；iOS 仍需自行签名。若需要其他 CPU 架构，可在不填写 tag 的前提下关闭 quick。

发布 job 的条件只在版本标签或非空 `tag` 输入时满足。上述命令只生成 Artifacts，`Publish GitHub Release` 应显示 skipped。安装包从对应 Actions run 的 Artifacts 下载，不会出现在 Releases 页面。旧 Release 保持不变。

合并、新仓库迁移、旧版清理和正式发布由维护者另行决定。
