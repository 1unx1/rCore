# rCore-Lab1

曹伦郗 *2020011020*

#### 功能实现

##### 定义

我定义了一个结构体`TaskInnerInfo`保存了某任务的：

- 各个系统调用次数；
- 第一次被调度时刻。

为它实现了方法`zero_init`用于初始化，将系统调用次数设置为0，第一次被调度时刻设置为`Option::None`。

##### 维护

我将这个结构体添加到了`TaskControlBlock`的成员中，用于保存该任务的这些信息，具体维护操作为：

- 当某任务在`run_first_task`或`run_next_task`中第一次被调度（通过判断是否为`Option::None`来判断是否是第一次）时，用`get_time_us`将被调度时刻记录下来；

- 在进入`trap_handler`之后，调用`syscall`之前，通过新增的方法和函数（`TaskManager.update_current_syscall_times`和`update_current_syscall_times`），更新对应的系统调用次数。

##### 使用

当发生`sys_task_info`系统调用时，该系统调用函数会通过新增的方法和函数（`TaskManager.get_current_info`和`get_current_info`），获取该任务的`TaskInnerInfo`。再由`get_time_us`获取当前时刻，就获得了足够的信息以构造出所需的`TaskInfo`实例。

#### 简答作业

1. 我使用的SBI信息：

   ![image-20230307220416873](C:\Users\19652\AppData\Roaming\Typora\typora-user-images\image-20230307220416873.png)

   - ch2b_bad_address.rs

     该用户程序对地址`0x0`进行了访问，触发了缺页异常。

     ![image-20230307214756674](C:\Users\19652\AppData\Roaming\Typora\typora-user-images\image-20230307214756674.png)

   - ch2b_bad_instructions.rs

     该用户程序中含有内核态S的特权级指令`sret`，CPU在用户态执行该指令会产生非法指令异常。

     ![image-20230307214906101](C:\Users\19652\AppData\Roaming\Typora\typora-user-images\image-20230307214906101.png)

   - ch2b_bad_register.rs

     该用户程序中含有访问内核态S下才能访问的CSR`sstatus`，CPU在用户态执行该指令会产生非法指令异常。

   ![image-20230307214943115](C:\Users\19652\AppData\Roaming\Typora\typora-user-images\image-20230307214943115.png)

2. (1)

   刚进入`__restore`时，`a0`代表：

   - 当某任务发生系统调用时，它代表此任务的Trap上下文的地址；
   - 当某任务发生中断或异常时，它代表此任务的Task上下文的地址。

   `__restore`有如下使用情景：

   - 由`__trap_handler`直接返回至`__restore`。

     当发生系统调用时，`__alltraps`调用`trap_handler`并返回后，就会跳转至`__restore`，以恢复Trap上下文，继续运行用户程序；

   - 由`__switch`直接返回至`__restore`。

     - 当发生中断或异常时`trap_handler`会调用`__switch`以保存当前任务上下文并恢复下一任务上下文。

     - 运行第一个任务时`run_first_task`也会调用`__switch`，用于加载第一个任务的上下文。
     
     而`__switch`将会返回至`__restore`，以恢复Trap上下文，运行下一任务，或运行第一个任务。

   ---

   (2)

   特殊处理了`sstatus`、`sepc`、`sscratch`。

   在`__restore`中，对于进入用户态，它们的意义分别是：

   - `sstatus`的`SPP`字段保存了特权级切换前的特权级，`sret`会将当前特权级设置为该字段记录的特权级，在此处即回到用户态U；

   - `sepc`保存了Trap处理完毕后，下一条要执行的指令的地址，为`sret`提供了所要跳转到的地址；

   - `sscratch`保存了用户栈栈顶地址，在进入用户态之前，它会与此时保存了内核栈栈顶地址`sp`交换，使`sp`重新指向用户栈栈顶，而`sscratch`自身指向内核栈栈顶，恢复进入Trap之前的状态。

   ---

   (3)

   `x2`即`sp`寄存器，它此时指向内核栈栈顶，即保存了Trap上下文的地址，而它所要恢复的值保存在`sscratch`中。

   因此，Trap上下文中其他寄存器的恢复，以及`sscratch`的恢复，依赖于`sp`当前的值。因此`sp`应当在Trap上下文中其他寄存器恢复都完成后，再释放内核栈中Trap上下文占用的空间，并通过和`sscratch`交换值来恢复自己。

   `x4`即`tp`寄存器，除非我们手动出于一些特殊用途使用它，一般不会被用到，所以无需恢复。

   ---

   (4)

   执行该指令之后，`sp`中的值是进入Trap前的用户栈栈顶地址，`sscratch`中的值是进入Trap前的内核栈栈顶地址。

   ---

   (5)

   状态切换发生在`sret`指令。因为该指令有如下功能：

   - 将当前特权级设置为`sstatus`的`SPP`字段保存的特权级U或S，在此处即用户态U；
   - 跳转到`sepc`所保存的下一条要执行的指令的地址，在此处该指令应该是一条用户程序中的指令。

   因此，执行`sret`后，CPU会进入用户态。

   ---

   (6)

   执行该指令之后，`sp`中的值是进入Trap前的内核栈栈顶地址，`sscratch`中的值是进入Trap前的用户栈栈顶地址。

   ---

   (7)

   在Trap发生的一瞬间，从U态进入S态。故应该是触发Trap的指令。

#### 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > 无

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > [rCore-Tutorial-Book-v3 3.6.0-alpha.1 文档](https://learningos.github.io/rCore-Tutorial-Book-v3/index.html#)
   >
   > [rCore-Tutorial-Guide-2023S 文档 (learningos.github.io)](https://learningos.github.io/rCore-Tutorial-Guide-2023S/index.html)

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
