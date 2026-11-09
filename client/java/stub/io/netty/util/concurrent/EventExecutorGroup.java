package io.netty.util.concurrent;
public interface EventExecutorGroup {
    ScheduledFuture<?> schedule(Runnable command, long delay, java.util.concurrent.TimeUnit unit);
}
