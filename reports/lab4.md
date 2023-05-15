# rCore-Lab4

曹伦郗 *2020011020*

#### 功能实现

##### sys_fstat

1. 实现`Inode::get_fstat`：
   1. 找出名为`name`的目录项，得到`inode_id`。
   2. 遍历各项求`inode_id`出现次数`nlink`。
2. 在进程控制块中新增成员`name_tb`，`name_tb[_fd]`即文件描述符为`_fd`的文件名。
3. 由`_fd`得到所查文件名，用`ROOT_INODE`调用`get_fstat`得到`inode_id`和`nlink`，构造`Stat`。

##### sys_linkat

1. 实现`Inode::link_at`：
   1. 通过`find_inode_id`找出名为`old_name`的目录项的`inode_id`。
   2. 通过`increase_size`以新增一个目录项的空间。
   3. 添加新目录项`{new_name, inode_id}`。
2. 用`ROOT_INODE`调用`link_at`。

##### sys_unlinkat

1. 实现`Inode::unlink_at`：
   1. 找出名为`name`的目录项，将其之后的各项依次向前移动一步。
   2. `Inode.size`减1。
2. 用`ROOT_INODE`调用`unlink_at`。

#### 简答作业

- > 在我们的easy-fs中，root inode起着什么作用？如果root inode中的内容损坏了，会发生什么？

  *easy-fs*是一个扁平化的文件系统，目录树上仅有一个目录，即根节点目录，即`ROOT_INODE`。故每个文件都是`ROOT_INODE`的目录项，它是我们打开文件，创建/取消文件硬链接，获取文件状态的直接操作对象。

  如果`ROOT_NODE`中内容损坏，我们将可能无法正确地操作文件。

---

- > 举出使用 pipe 的一个实际应用的例子。

  以用`cat`和`wc`完成一个文件的行数统计为例：

  ```shell
  cat file.txt | wc -l
  ```

  该命令中`pipe`将`cat`的输出作为`wc`的输入，`wc -l`统计其输入的行数，输出到标准输出，即得到了文件的行数。

---

- > 如果需要在多个进程间互相通信，则需要为每一对进程建立一个管道，非常繁琐，请设计一个更易用的多进程通信机制。

  可以使用消息队列实现更简洁的多进程通信。

#### 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > 无

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > [rCore-Tutorial-Book-v3 3.6.0-alpha.1 文档](https://learningos.github.io/rCore-Tutorial-Book-v3/index.html#)
   >
   > [rCore-Tutorial-Guide-2023S 文档 (learningos.github.io)](https://learningos.github.io/rCore-Tutorial-Guide-2023S/index.html)

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。