 
 
 
package io.netty.channel;

public interface ChannelHandlerContext {
    ChannelFuture write(Object msg, ChannelPromise promise);

    ChannelHandlerContext fireChannelRead(Object msg);
    ChannelHandlerContext flush();
    ChannelHandlerContext fireChannelInactive();
    io.netty.util.concurrent.EventExecutor executor();
}
