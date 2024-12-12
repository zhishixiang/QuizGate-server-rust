# TestGate

一个基于答题验证的Minecraft服务器白名单准入系统。

## 项目简介

TestPass 是一个专为 Minecraft 服务器设计的白名单管理系统。通过完成指定的测试题目，玩家可以获得服务器的准入资格。这种方式既能确保玩家对服务器规则的理解，也能提升社区质量。

本项目目前仍然处于内测阶段，且需要搭配特定客户端使用。如果您对本项目感兴趣，欢迎提交 Issue 和 Pull Request。

##  快速开始

您可以通过自托管方式使用本项目，仅需以下几步：

1.下载Release页面中对应平台的最新版本。
2.在根目录下新建以下形式的配置文件并命名为config.toml:
```toml
# 是否为本地自托管模式
self_hosted = true
# 本地模式下的key
self_hosted_key = "local_key"
# ip地址，如果向局域网或公网开放请填写0.0.0.0
address = "127.0.0.1"
# 端口
port = 8081
```
3.将问卷文件重命名为0.json并放入根目录。
4.运行程序，访问配置好的地址，如果一切正常，您将看到一个问卷页面。
5.配置客户端，将服务器地址设置为您的服务器地址的ws路径，比如地址为127.0.0.1:8080就设置为ws://127.0.0.1:8080/ws，将key设置为配置文件中的key。

## 开源协议

本项目采用 MIT 协议开源。