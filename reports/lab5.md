# rCore-Lab5

曹伦郗 *2020011020*

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