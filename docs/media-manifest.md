# 媒体清单协议 v1

这是一个只读、声明式 JSON 协议。清单没有可执行脚本、任意请求头、解密密钥或账号接口。客户端检验格式和访问策略；内容使用权需要由提供方与用户确认。

```json
{
  "schema_version": 1,
  "id": "my-library",
  "name": "我的媒体库",
  "items": [
    {
      "id": 1,
      "title": "我的旅行记录",
      "description": "我拍摄并拥有的素材",
      "tags": ["旅行"],
      "author": "我",
      "episodes": [
        {
          "title": "第一段",
          "url": "https://media.example.com/owned-video.mp4",
          "format": "file"
        }
      ]
    }
  ]
}
```

示例域名是占位符，请替换成你有权使用的服务器地址。你可以让 HGhubX 直接连接清单 JSON，也可以从网站主页自动发现这个清单。

## 从网站自动发现

网站可以在 HTML 的 `head` 中使用以下任一方式声明清单：

```html
<link rel="alternate"
      type="application/vnd.hghubx.manifest+json"
      href="/library.json">
```

```html
<link rel="hghubx-manifest" href="/library.json">
<meta name="hghubx-manifest" content="/library.json">
```

如果页面没有声明，HGhubX 还会依次尝试同源公开路径：

- `/.well-known/hghubx.json`
- `/hghubx.json`
- `/library.json`

HGhubX 只读取 HTML 标记并连接返回媒体清单 JSON 的同源候选地址。它不会执行网页脚本、跟随页面中的普通链接，也不会调用登录后的私有接口。声明或约定路径返回的实际清单仍然遵循下文的格式、URL 和网络限制。若清单位于不同来源，请直接输入清单 JSON 地址。

## 字段与限制

- `schema_version` 必须为 `1`。
- `id` 是该清单的稳定标识，1–128 字节；导入文件时用于区分媒体库。为不同的导入清单使用不同标识，更新同一清单时保留标识。
- `name` 是显示名称，1–256 字节。
- `items` 包含 1–2000 个条目。每个条目的 `id` 必须是唯一的正整数，最大 `9007199254740991`；请保持 ID 稳定，避免修改文件顺序时丢失进度对应关系。
- 条目的 `title` 必填；`description`、`cover`、`tags`、`author` 可选。封面支持普通 JPEG、PNG、GIF、WebP，不支持带脚本的 SVG。
- 每个条目包含 1–1000 个 `episodes`，整个清单最多 10000 集。`url` 必填；`format` 可选，支持 `auto`、`file`、`hls`。`auto` 按 URL 路径的 `.m3u8` 后缀识别 HLS，其他地址按普通媒体文件播放；无后缀的 HLS 需显式填写 `hls`。
- `duration` 可选，为非负秒数；未填写时由播放器读取真实时长。
- 清单最大 2 MiB，拒绝不认识的字段。URL 仅允许不带用户名与密码的 HTTP(S) 地址；本机路径、`file:`、`data:`、`javascript:` 不被远程清单接受。
- 远程清单的相对媒体地址按清单最终 URL 解析。导入文件中的媒体地址必须为绝对地址；不会把导入文件当作任意本机文件读取权限。

## 网络与播放

默认检查域名、解析 IP、每一跳重定向和 HTTPS 降级。需要自建局域网时，由用户在导入或连接时明确勾选授权。该授权只用于当前清单和清单列出的媒体来源；HLS 引用未列出的其他内网主机仍会被阻止。HTTP 地址不适合传递敏感凭据。

普通视频优先使用 HTTP Range，单次最多缓冲 8 MiB。服务器不支持 Range 时，只接受不超过 32 MiB 的整文件响应。HLS 分片及图片的单次响应也有上限，避免无界下载。

导入后的能力由协议确定：浏览、搜索、标签、相关推荐，以及本机收藏与进度。清单不能声明登录、评论、帖子、远程点赞、可执行适配器或全局安全豁免。
