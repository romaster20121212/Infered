import java.util.ArrayDeque;
import java.util.concurrent.TimeUnit;

 
public class InferedChannelHandler extends io.netty.channel.ChannelDuplexHandler implements Runnable {
    private static final long POLL_NANOS = TimeUnit.MILLISECONDS.toNanos(10);
    private static final int MAX_QUEUED = 2048;

    public static volatile int OUTBOUND_DELAY_MS;
     




    public static volatile boolean STRICT_OUTBOUND;
    private static final ThreadLocal<Boolean> IMMEDIATE = ThreadLocal.withInitial(() -> false);

     
    public static void sendImmediately() {
        IMMEDIATE.set(true);
    }

    private final ArrayDeque<Object> messages = new ArrayDeque<>();
    private final ArrayDeque<io.netty.channel.ChannelPromise> promises = new ArrayDeque<>();
    private final ArrayDeque<Long> deadlines = new ArrayDeque<>();
    private final ArrayDeque<Long> arrivals = new ArrayDeque<>();
    private io.netty.channel.ChannelHandlerContext context;
    private boolean scheduled;
    private long lastArrival;
    private long lastRelease;

    private static native Object onOutbound(Object packet);
    private static native Object onInbound(Object packet);

    protected Object dispatchOutbound(Object packet) { return onOutbound(packet); }
    protected Object dispatchInbound(Object packet) { return onInbound(packet); }

    private static long delayNanos() {
        return TimeUnit.MILLISECONDS.toNanos(Math.max(0, Math.min(1000, OUTBOUND_DELAY_MS)));
    }

    private long releaseTime() {
        long due = delayNanos() == 0 ? 0 : deadlines.peekFirst();
         
         
        if (!STRICT_OUTBOUND && lastRelease != 0) {
            long spacing = Math.max(0, arrivals.peekFirst() - lastArrival);
            if (spacing < TimeUnit.MILLISECONDS.toNanos(10)) spacing = 0;
            due = Math.max(due, lastRelease + spacing);
        }
        return due;
    }

    private void drainDue() {
        boolean wrote = false;
        while (!messages.isEmpty()) {
            long now = System.nanoTime();
            if (releaseTime() > now) break;
            deadlines.removeFirst();
            lastArrival = arrivals.removeFirst();
            context.write(messages.removeFirst(), promises.removeFirst());
            lastRelease = now;
            wrote = true;
        }
        if (wrote) context.flush();
        if (messages.isEmpty()) lastArrival = lastRelease = 0;
    }

    private void drainAll() {
        boolean wrote = false;
        while (!messages.isEmpty()) {
            deadlines.removeFirst();
            arrivals.removeFirst();
            context.write(messages.removeFirst(), promises.removeFirst());
            wrote = true;
        }
        lastArrival = lastRelease = 0;
        if (wrote) context.flush();
    }

    private void scheduleDrain() {
        if (scheduled || messages.isEmpty()) return;
        long wait = Math.max(1, Math.min(POLL_NANOS, releaseTime() - System.nanoTime()));
        scheduled = true;
        context.executor().schedule(this, wait, TimeUnit.NANOSECONDS);
    }

    @Override public void run() {
        scheduled = false;
        drainDue();
        scheduleDrain();
    }

    @Override public void write(io.netty.channel.ChannelHandlerContext ctx, Object msg,
                                io.netty.channel.ChannelPromise promise) throws Exception {
        context = ctx;
        IMMEDIATE.remove();
        Object result;
        boolean immediate;
        try {
            result = dispatchOutbound(msg);
            immediate = IMMEDIATE.get();
        } finally {
            IMMEDIATE.remove();
        }
        if (result == null) { promise.trySuccess(); return; }
        if (immediate && !STRICT_OUTBOUND) {
            ctx.write(result, promise);
            ctx.flush();
            return;
        }
        long delay = delayNanos();
        if (delay == 0 && messages.isEmpty()) {
            ctx.write(result, promise);
            return;
        }
         
        if (messages.size() >= MAX_QUEUED) drainAll();
        long now = System.nanoTime();
        long due = now + delay;
        if (!deadlines.isEmpty()) {
            due = Math.max(due, deadlines.peekLast());
        }
        messages.addLast(result);
        promises.addLast(promise);
        deadlines.addLast(due);
        arrivals.addLast(now);
        scheduleDrain();
    }

    @Override public void channelRead(io.netty.channel.ChannelHandlerContext ctx, Object msg) throws Exception {
        context = ctx;
        Object result = dispatchInbound(msg);
        if (result != null) ctx.fireChannelRead(result);
    }

    @Override public void handlerRemoved(io.netty.channel.ChannelHandlerContext ctx) throws Exception {
        context = ctx;
        drainAll();
    }

    @Override public void channelInactive(io.netty.channel.ChannelHandlerContext ctx) throws Exception {
        while (!promises.isEmpty()) promises.removeFirst().tryFailure(new IllegalStateException("Connection closed"));
        messages.clear();
        deadlines.clear();
        arrivals.clear();
        lastArrival = lastRelease = 0;
        ctx.fireChannelInactive();
    }
}
