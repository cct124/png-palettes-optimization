# png-palettes-optimization

> **png&apng 命令行有损压缩工具**

将 `PNG` 图像的 RGBA 模式转为调色板模式，从而减小图像大小。目前压缩只压缩 RGBA 模式的图像，调色板模式将跳过。这是一个多线程有损`PNG`图像压缩工具。

## 使用方法

```shell
# 查看命令
png-palettes-optimization -help
```

> 使用[libimagequant](https://github.com/ImageOptim/libimagequant)生成调色板，
> [image-png](https://github.com/image-rs/image-png)解码编码`PNG`图像数据
