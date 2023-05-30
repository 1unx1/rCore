# rCore-Lab1

曹伦郗 *2020011020*

#### 功能实现

##### Spawn

实现了`TaskControlBlock::spawn`，主要有3个步骤：

1. 仿照`TaskControlBlock::new`根据ELF文件新建一个进程，为其分配新的进程号和内核栈，地址空间通过调用读取ELF文件获得。
2. 仿照`TaskControlBlock::fork`，将新建进程添加到当前进程的子进程中。
3. 仿照`TaskControlBlock::exec`，对新建进程的Trap上下文进行初始化，其中的`sepc`和`sp`值通过读取ELF文件得到，内核栈指针为新分配的内核栈栈顶地址。

##### Stride Algorithm

在`TaskControlBlock.inner`中保存了`prio`和`stride`，修改了`TaskManager::fetch`，遍历队列取出`stride`最小的进程，并更新其`stride`。

#### 问答作业

- > 实际情况是轮到p1执行吗？为什么？

  不是。因为p2.stride叠加一个pass之后发生了上溢，更新后的p2.stride = 4又比p1.stride = 255更小，故下一次调度仍是p2执行。

- > 为什么？尝试简单说明。

  在不考虑溢出的情况下，当任意进程的优先级$P_i.priority$都满足$P_i.priority≥2$，则$P_i.pass=BigStride/P_i.priority≤BigStride/2$。

  下面用归纳法进行证明。

  1. 假设当第$k$次调度完成后，$max\{P_i.stride\}-min\{P_i.stride\}≤BigStride/2···(*)$。

     不妨设$max\{P_i.stride\}=P_m.stride$，$min\{P_i.stride\}=P_n.stride$。

     则第$k+1$次调度选择$P_n$执行。

     更新后的$P_n'.stride=P_n.stride+P_n.pass≤P_n.stride+BigStride/2$。

     1. 若$P_n'.stride≤P_m.stride$。

        则更新后，$max\{P_i.stride\}$不变，$min\{P_i.stride\}$增大，则不等式(\*)显然仍然成立。

     2. 若$P_n'.stride>P_m.stride$。

        则更新后，$max\{P_i.stride\}=P_n'.stride$，$min\{P_i.stride\}≥P_n.stride$。

        故$max\{P_i.stride\}-min\{P_i.stride\}≤P_n'.stride-P_n.stride=P_n.pass≤BigStride/2$，即不等式(\*)仍然成立。

     故若当第k次调度完成后，不等式(\*)成立，则当第k+1次调度完成后，不等式(\*)仍然成立。
  
  2. 当第$k=0$次调度完成后，即初始状态下，$P_i.stride=0$，不等式(*)显然成立。
  
  由1&2，在不考虑溢出的情况下，当任意进程的优先级$P_i.priority$都满足$P_i.priority≥2$，则$max\{P_i.stride\}-min\{P_i.stride\}≤BigStride/2$。
  
- > 补全下列代码中的`portial_cmp`函数，假设两个Stride永远不会相等。

  ```rust
  impl PartialOrd for Stride {
      fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
          if self.0 < other.0 {
              if other.0 - self.0 <= u64::MAX / 2 {
                  Some(Ordering::Less)
              } else {
                  // Overflow
                  Some(Ordering::Greater)
              }
          } else {
              if self.0 - other.0 <= u64::MAX / 2 {
                  Some(Ordering::Greater)
              } else {
                  // Overflow
                  Some(Ordering::Less)
              }
          }
      }
  }
  ```

#### 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > 无

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > [rCore-Tutorial-Book-v3 3.6.0-alpha.1 文档](https://learningos.github.io/rCore-Tutorial-Book-v3/index.html#)
   >
   > [rCore-Tutorial-Guide-2023S 文档 (learningos.github.io)](https://learningos.github.io/rCore-Tutorial-Guide-2023S/index.html)

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
