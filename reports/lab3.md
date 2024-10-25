# 1.总结
这章加入了进程的概念，是的不再只是任务调度，而是基于进程提供任务。我实现了sys_spawn系统调用，该调用不需要复制当前任
务的内容，减小了开销,直接创建了新任务并执行文件，但新进程仍有相应的父进程。然后我实现了stride的简单调度算法,我在TaskControlBlock的
TaskControlBlockInner里加入了基础条目来辅助实现。我为了不破坏框架仍使用了那个队列来为那一个进程保存任务队列，具体实现时，我将每个任务取出在筛选出stride值最小的任务执行，其余
放回队列。额外地，我实现了溢出处理，即当队列里若所有的stride值都等于了BIG_STRIDE 就会重置每个任务的stride的值为0。

# 2.简答作业
## Q1.1
不会，因为u8类型溢出后rust的处理好像是会回环形式的重新赋值，比如这里的p2.stride可能会变成4之类的，因此下次可能不会轮到p1执行。

## Q2.1.1
最大步长值STRIDE_MIN为BigStride / 2，最小步长值STRIDE_MAX为BigStride / P，P为优先级。在进程优先级全部 >= 2 的情况下，这个表达式总是成立的，这样也确保了步长值的差距不会太大，保证了调度的公平性和效率。

## Q2.1.2
代码如下，默认最小特权级>=2
```rust
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		let overflow_flag:bool = false; 
		if -BIG_STRIDE/2 <= self.0 - other.0 && self.0 - other.0 <= BIG_STRIDE/2 {
			overflow_flag = false;
		} else {
			overflow_flag = true;
		}
		match overflow_flag {
			true => {
				if self.0 > other.0 {
					return Some(Ordering::Greater);
				} else {
					return Some(Ordering::Less);
				}
			},
            false => {
				if self.0 < other.0 {
					return Some(Ordering::Greater);
				} else {
					return Some(Ordering::Less);
				}
			},
		}
	}
}
```
# 3.看法
在任务调度的基础上想学习进程调度以及想了解多核编程。

# 荣誉准则：
我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
