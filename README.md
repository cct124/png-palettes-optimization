# png-palettes-optimization

> **PNG&APNG 命令行有损压缩工具**

将 `PNG` 图像的 RGBA 模式转为调色板模式，从而减小图像大小。目前压缩只压缩 RGBA 模式的图像，调色板模式将跳过。这是一个多线程有损`PNG`图像压缩工具。

## 使用方法

```shell
# 执行程序，将压缩当前目录下所有的PNG图像
png-palettes-optimization
```

## 运行参数

```shell
# 查看命令
png-palettes-optimization --help

Options:
  -p, --path <PATH>
          要进行压缩png图的文件夹路径，传入当前工作路径的相对路径。 默认当前工作路径
  -s, --speed <SPEED>
          1-10，更快的速度生成的图像质量更低，可用于实时生成图像。默认值为 4
  -n, --quality-min <QUALITY_MIN>
          0-100，优化的最低质量，默认最低 0，不能高于最大值
  -x, --quality-max <QUALITY_MAX>
          0-100，优化的最大质量，默认最高100，不能低于最小值
  -d, --dithering-level <DITHERING_LEVEL>
          设置为1.0可获得漂亮的平滑图像，默认 1.0
  -c, --compression <COMPRESSION>
          施加压缩的类型和强度，三种类型default、fast、equal，默认default，最好的压缩但时间会更长 [possible values: default, fast, equal]
  -e, --exclude <EXCLUDE>
          压缩时需要排除的文件，传入PNG文件名
  -h, --help
          Print help information
  -V, --version
          Print version information
```

> 使用[libimagequant](https://github.com/ImageOptim/libimagequant)生成调色板，
> [image-png](https://github.com/image-rs/image-png)解码编码`PNG`图像数据

## 示例

```shell
# 执行程序，将压缩当前目录下所有的PNG图像
png-palettes-optimization

# 设置压缩速度
png-palettes-optimization -s 1

# 需要排除压缩的PNG文件
png-palettes-optimization -e test_1.png -e test_2.png

# 设置压缩质量，-x 为最大质量，压缩时将尽量接近设置的质量。如果想要更好的图像质量可以传入参数 -s 1 -x 100
png-palettes-optimization -x 99
```
