RSHILED

这是一个tauri框架开发的的桌面应用程序，主要目的是用于漏洞挖掘和bughunter使用

## 功能
-ASM（攻击面管理）
-漏洞的主被动扫描
-插件管理
-POC管理（nuclei）
-暴破模块
-专项漏洞检测
-系统设置


## ASM主要包含
- 信息总览
- 企业信息（待测试的企业，SRC，或bughunter）
- 种子域名（根域名）
- 域名解析 （通过暴破、信息收集到的域名信息）
- IP地址 （域名对应的域名解析记录信息）
- 端口信息 （IP开放的端口信息）
- 网站信息 （企业暴破的网站）
- API接口 （网站中的API信息）
- WEB组件（指纹信息）
- 安全风险（漏洞、敏感信息泄露，文件泄露等等）

## 扫描相关
- 主动扫描
	给定URL或URL列表，通过POC和插件进行扫描
- 被动扫描
	通过监听端口对通过的流量进行漏洞检测，包含但不限于XSS、SQL注入、REC等等

## 暴破模块
- 服务暴破
	包含但不限于 SSH、MYSQL、UDP、常见应用后台暴破（需要更灵活的配置）

## 插件模块
- 插件配置
	创建和修改插件信息，存于数据库SQlite
	提供给如ASM，暴破模块、和扫描模块等使用

## 系统配置
主要存取本应用的各种配置，配置信息存到数据库中的config表中


## 技术栈
- SQLite
- arco-design
- rust
- vue
- boa javascritp runtime
- ast

请读取项目信息，其中部分模块已经完成，请针对未完成的模块进行开发



### rsubdomain的问题
需要安装wireshark，然后安装ChmodBPF，手动运行如下命令让效果持久
使用时需要关闭tun代理模式
```
sudo launchctl enable system/org.wireshark.ChmodBPF
sudo launchctl load '/Library/LaunchDaemons/org.wireshark.ChmodBPF.plist'
```