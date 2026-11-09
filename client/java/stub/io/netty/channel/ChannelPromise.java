 
 
package io.netty.channel;

public interface ChannelPromise {
    boolean trySuccess();
    boolean tryFailure(Throwable cause);
}
