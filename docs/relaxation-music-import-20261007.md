# 舒缓音乐导入记录与前端接入

已从作者官网 [Incompetech](https://incompetech.com/music/royalty-free/) 下载并导入 HP 的 `digital_companion.music`。首批 3 首，后续再扩充 17 首，当前共 **20 首**，总时长 **64 分 35 秒**，全部 `status = 1`，分类为 `relaxation`，音频 MIME 类型为 `audio/mpeg`。

完整 20 首的 musicId、来源、许可、大小与哈希见 [曲库清单](../database/music/relaxation-library-20261007.json)。以下是首批记录：

| musicId | 展示标题 | 原曲 | 时长 | 音频大小 |
| --- | --- | --- | --- | --- |
| 1 | 舒缓放松 · 钢琴冥想 01 | Meditation Impromptu 01 | 3:33 | 7,042,622 字节 |
| 2 | 舒缓放松 · 钢琴冥想 02 | Meditation Impromptu 02 | 4:09 | 8,261,798 字节 |
| 3 | 舒缓放松 · 宁静心绪 | Clean Soul | 5:07 | 12,271,405 字节 |

前两首为钢琴音乐，第三首为电钢琴与低音的轻环境音乐。按作者目录的 Calming/Relaxed 描述选择，用于系统中的放松、休息及日记背景场景；未剪辑、未重新编码，只添加中文展示标题。

## 新增 17 首

| musicId | 展示标题 | 原曲 | 时长 |
| --- | --- | --- | --- |
| 4 | 舒缓放松 · 晨光 | Morning | 2:33 |
| 5 | 舒缓放松 · 暮色 | Evening | 3:06 |
| 6 | 舒缓放松 · 轻柔电钢琴 01 | Mana Two - Part 1 | 3:54 |
| 7 | 舒缓放松 · 轻柔电钢琴 03 | Mana Two - Part 3 | 3:12 |
| 8 | 舒缓放松 · 夜间故事 | Midnight Tale | 2:42 |
| 9 | 舒缓放松 · 樱花 | Cherry Blossom | 2:10 |
| 10 | 舒缓放松 · 清澈流水 | Clear Waters | 2:08 |
| 11 | 舒缓放松 · 周五清晨 | Friday Morning | 3:30 |
| 12 | 舒缓放松 · 钢琴冥想 03 | Meditation Impromptu 03 | 4:15 |
| 13 | 舒缓放松 · 月光石 | Moonstone | 4:15 |
| 14 | 舒缓放松 · 温柔回忆 | Reminiscing | 2:32 |
| 15 | 舒缓放松 · 蓝宝石岛 | Sapphire Isle | 2:12 |
| 16 | 舒缓放松 · 星夜 | Starry | 3:19 |
| 17 | 舒缓放松 · 温柔时光 | There is Romance | 3:18 |
| 18 | 舒缓放松 · 留白 | White | 3:05 |
| 19 | 舒缓放松 · 冬日铃声 | Winter Chimes | 2:38 |
| 20 | 舒缓放松 · 静思 | Deliberate Thought | 2:57 |

扩充加入了吉他、竖琴、长笛、电钢琴、马林巴和轻环境音色。来源及许可与首批相同，逐曲元数据见 [扩充清单](../database/music/relaxation-expansion-20261007.json)。

## 来源与许可

作者均为 Kevin MacLeod（incompetech.com），免费许可为 [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)，作者的 [免费许可说明](https://incompetech.com/music/royalty-free/licenses/) 要求署名。

- [Meditation Impromptu 01 原曲页面](https://incompetech.com/music/royalty-free/index.html?isrc=USUAN1100163)
- [Meditation Impromptu 02 原曲页面](https://incompetech.com/music/royalty-free/index.html?isrc=USUAN1100162)
- [Clean Soul 原曲页面](https://incompetech.com/music/royalty-free/index.html?isrc=USUAN1300033)

**前端曲目详情或播放页需要展示作者、原曲名、来源及许可链接。** 这些信息已写入数据库 `description`，音乐列表和详情接口均返回该字段。不要只展示中文标题而隐藏署名。可将整段 description 作为文本展示，再给来源和许可提供可点击链接；不要将远程文本直接当 HTML 注入。

完整来源、下载地址、许可、修改说明、音频哈希及实际 musicId 见 [导入清单](../database/music/relaxation-20261007.json)。后续环境应从列表取得 musicId，不假设其他数据库也使用 1、2、3。

## 前端请求

baseUrl 使用实际应用部署地址。音乐接口为公开 GET：

| 用途 | 地址 | SDK |
| --- | --- | --- |
| 曲目列表 | `/api/v1/music/tracks?category=relaxation&page=1&pageSize=20` | `client.music.tracks({ category: 'relaxation', page: 1, pageSize: 20 })` |
| 曲目详情 | `/api/v1/music/tracks/{musicId}` | `client.music.track(musicId)` |
| MP3 音频 | `/api/v1/music/tracks/{musicId}/stream` | `client.music.stream(musicId)`，返回 Blob |

播放代码示例，client 为已经初始化的 UserClient：

```ts
const { items } = await client.music.tracks({ category: 'relaxation', page: 1, pageSize: 20 })
const selected = items[0]
if (selected) {
  // 同时展示 selected.title、artist 和 description（含原曲来源及许可）。
  const blob = await client.music.stream(selected.musicId)
  const url = URL.createObjectURL(blob)
  const audio = new Audio(url)
  audio.volume = 0.3
  // 用户点击播放时调用，异常交给页面处理。
  await audio.play()
  // 路由卸载、替换曲目时：
  // audio.pause()
  // audio.removeAttribute('src')
  // audio.load()
  // URL.revokeObjectURL(url)
}
```

也可使用 `<audio controls src="实际API地址/api/v1/music/tracks/1/stream">`。音频文件已存入项目数据库，用户播放不需要连接作者外站。当前流接口返回完整音频，不提供 Range 分段响应；上述 SDK 方式会先取得完整 Blob。

这次导入只增加曲库内容，不新增健康促进 activity，也不自动在用户评估后播放音乐。健康促进目前仍提供呼吸练习和情绪日记，音乐由前端音乐页或训练页按需要接入。

## 备份与验证

- 导入前单独备份 music 表，远程路径 `/home/x/ServerRSBackups/20261007_music_import/music_before.sql.gz`。
- 本地备份及音频原文件位于 `database/backups/20261007_music_import/`，目录不进入 Git。
- music 表备份 SHA-256：`7b6174cdcbfe4955d61aa266c57917b7c99ae59030fc9014ce54ba875b374bcf`，远端和本地一致。
- 全部 20 首音频均通过 ffprobe 检查及 ffmpeg 完整解码检查。
- 导入使用一个事务，存储形式遵循现有服务的 Base64 数据约定，file_size 为原音频字节数。
- 导入后在 MySQL 中用 FROM_BASE64 解码，逐首检查实际大小、SHA-256、时长、启用状态和许可信息，全部通过。
- 原曲目录及网站许可模板保存在本地备份目录，核对输出为 verification.jsonl。

扩充前又单独备份了原有 3 首曲库，本地目录为 `database/backups/20261007_music_expand/`，HP 为 `/home/x/ServerRSBackups/20261007_music_expand/`。该备份 SHA-256 为 `db0a605face328dc06e21da54056847ce67a61323ce66db388d98a8f6112608c`，本地和远程一致，gzip 完整性验证通过。新增 17 首事务导入后逐首核对解码哈希，全部匹配；最终 MySQL 健康检查正常，曲目数与启用数均为 20。

此次未部署应用，未验证部署后的真实 HTTP 播放接口。已确认音频数据与现有服务器解码逻辑兼容，应用连接该数据库并正常启动后可通过上述接口获取；最终浏览器播放需在部署环境联调。
