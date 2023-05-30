# rCore-Lab5

曹伦郗 *2020011020*

本次实验大约使用了2天，每天大约三小时。

#### 功能实现

1. 在`PCBInner`内加入了表示是否使能死锁检测的成员`en_deadlock_detect`，以及分别用于`Mutex`和`Semaphore`保存银行家算法所需信息的成员。并分别对`Mutex`和`Semaphore`实现了死锁检测函数。

2. `sys_enable_deadlock_detect`即将当前进程的`en_deadlock_detect`置为`true`或`false`。

3. 在`sys_mutex_lock`、`sys_semaphore_down`中，自增`need`后，检测死锁。

   若成功，则调用`Mutex::lock`，并更新`available`自减，`need`自减，`alloc`自增；

   若失败或禁用死锁检测，则直接返回`-0xDEAD`。

4. 在`sys_mutex_unlock`、`sys_semaphore_up`中，调用`Mutex::unlock`或`Semaphore::up`前，更新`available`自增，`alloc`自减。

5. 在`sys_thread_create`、`sys_mutex_create`、`sys_semaphore_create`三个系统调用中，更新或初始化`available`、`need`、`alloc`三项信息。

#### 简答作业

1. > 需要回收的资源有哪些？

   子线程的资源（包括子线程的线程号（TID）、用户栈、Trap上下文、内核栈等）、页表、文件描述符表、信号、锁、条件变量等。

   > 其他线程的 TaskControlBlock 可能在哪些位置被引用，分别是否需要回收，为什么？

   可能在以下结构中被引用：

   1. 锁、信号、条件变量的等待队列。不需要手动回收，因为锁、信号、条件变量都是属于整个进程的资源，因此当整个进程控制块被回收，这些资源也被回收，在这些资源的等待队列中的线程也被自动回收。
   2. 线程管理器的就绪队列。需要手动回收，因为线程管理器在整个进程控制块被回收后仍存在，无法自动回收。

2. 区别：

   - `Mutex1::unlock`中，无论`wait_queue`弹出的返回值是否为`None`，`locked`都会被设置为`false`，即必然释放锁，如果有等待的线程，则唤醒等待最久的线程。

   - `Mutex2::unlock`中，仅当`wait_queue`弹出了一个`None`时，`locked`才会被设置为`false`，即如果没有等待的线程才释放锁，否则仅唤醒等待最久的线程。

   问题：
   
   假如有被唤醒的线程：
   
   - 对于`Mutex1::unlock`，就绪队列中优先级最高的进程，经调度开始运行后能先于这个刚被唤醒的线程抢占到锁。
   - 对于`Mutex2::unlock`，这个刚被唤醒的线程会直接抢占到锁，而就绪队列中优先级最高的进程，经调度开始运行后，也不能抢占到锁。

#### 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > 无

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > [rCore-Tutorial-Book-v3 3.6.0-alpha.1 文档](https://learningos.github.io/rCore-Tutorial-Book-v3/index.html#)
   >
   > [rCore-Tutorial-Guide-2023S 文档 (learningos.github.io)](https://learningos.github.io/rCore-Tutorial-Guide-2023S/index.html)

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。