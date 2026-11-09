 
 
 
package io.netty.channel;

public class ChannelDuplexHandler {
    public void write(ChannelHandlerContext ctx, Object msg, ChannelPromise promise)
            throws Exception {}

    public void channelRead(ChannelHandlerContext ctx, Object msg) throws Exception {}
    public void handlerRemoved(ChannelHandlerContext ctx) throws Exception {}
    public void channelInactive(ChannelHandlerContext ctx) throws Exception {}
}
