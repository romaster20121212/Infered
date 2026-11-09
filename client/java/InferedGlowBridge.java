import java.lang.reflect.*;
import java.util.*;

 



public final class InferedGlowBridge {
    public static final int MAX_PLAYERS = 40;
    public static final int MAX_FLOATS = MAX_PLAYERS * 4096 * 3;
    private static ClassLoader gameLoader;
    private static Method getRenderer, updateState, getModel, setAngles, renderModel;
    private static Method translate, scale, transforms, rendererScale, offset;
    private static Constructor<?> matrixStack;
    private static Class<?> consumer;
    private static Field baseScale, bodyYaw, sleepDirection, eyeHeight;
    private static Method directionX, directionZ, vecX, vecY, vecZ;
    private static final float[] vertices = new float[MAX_FLOATS];
    private static final float[] quad = new float[12];
    private static final int[] TRIANGLES = {0,1,2,0,2,3};
    private static int floats, quadVertices;
    private static Object sink;
     

    private static Field layers;
    private static Class<?> armorLayer;
    private static Method renderLayer;
    private static Field walkPos, walkSpeed;
    private static Object provider;
     

    private static boolean armorBroken;
    private InferedGlowBridge() {}

    private static void init(ClassLoader cl) throws Exception {
        if (gameLoader == cl) return;
        Class<?> entity=Class.forName("net.minecraft.class_1297",true,cl);
        Class<?> dispatcher=Class.forName("net.minecraft.class_898",true,cl);
        Class<?> renderer=Class.forName("net.minecraft.class_897",true,cl);
        Class<?> living=Class.forName("net.minecraft.class_922",true,cl);
        Class<?> state=Class.forName("net.minecraft.class_10017",true,cl);
        Class<?> livingState=Class.forName("net.minecraft.class_10042",true,cl);
        Class<?> stack=Class.forName("net.minecraft.class_4587",true,cl);
        Class<?> model=Class.forName("net.minecraft.class_583",true,cl);
        Class<?> direction=Class.forName("net.minecraft.class_2350",true,cl);
        Class<?> vec=Class.forName("net.minecraft.class_243",true,cl);
        consumer=Class.forName("net.minecraft.class_4588",true,cl);
        getRenderer=dispatcher.getMethod("method_3953",entity);
        updateState=renderer.getMethod("method_62425",entity,float.class);
        getModel=living.getMethod("method_4038");
        setAngles=model.getMethod("method_2819",state);
        renderModel=model.getMethod("method_62100",stack,consumer,int.class,int.class,int.class);
        matrixStack=stack.getConstructor();
        translate=stack.getMethod("method_22904",double.class,double.class,double.class);
        scale=stack.getMethod("method_22905",float.class,float.class,float.class);
        transforms=living.getDeclaredMethod("method_4058",livingState,stack,float.class,float.class);
        transforms.setAccessible(true);
        rendererScale=living.getDeclaredMethod("method_4042",livingState,stack); rendererScale.setAccessible(true);
        offset=renderer.getMethod("method_23169",state);
        baseScale=livingState.getField("field_53453"); bodyYaw=livingState.getField("field_53446");
        sleepDirection=livingState.getField("field_53463"); eyeHeight=state.getField("field_53331");
        directionX=direction.getMethod("method_10148"); directionZ=direction.getMethod("method_10165");
        vecX=vec.getMethod("method_10216");vecY=vec.getMethod("method_10214");vecZ=vec.getMethod("method_10215");
        sink=Proxy.newProxyInstance(cl,new Class<?>[]{consumer},(proxy,method,args)->{
             
            if(method.isDefault() && !method.getName().equals("method_23919"))
                return InvocationHandler.invokeDefault(proxy,method,args);
            if(method.getName().equals("method_22912") || method.getName().equals("method_23919")) {
                addVertex(((Number)args[0]).floatValue(),((Number)args[1]).floatValue(),((Number)args[2]).floatValue());
                return method.getReturnType()==void.class ? null : proxy;
            }
            if(method.getName().equals("toString"))return "InferedPlayerGeometry";
            if(method.getName().equals("hashCode"))return System.identityHashCode(proxy);
            if(method.getName().equals("equals"))return proxy==args[0];
            if(method.getReturnType()==consumer)return proxy;  
            throw new UnsupportedOperationException(method.toString());
        });
         
         
         
         
         
         
         
         
         
        try {
            Class<?> multiBuffer = Class.forName("net.minecraft.class_4597",true,cl);
            Class<?> entityStates=Class.forName("net.minecraft.class_10017",true,cl);
            Field layerField=living.getDeclaredField("field_4738");layerField.setAccessible(true);
            Class<?> layerClass=Class.forName("net.minecraft.class_970",true,cl);
            Method render=layerClass.getMethod("method_4199",stack,multiBuffer,int.class,entityStates,float.class,float.class);
            Field pos=livingState.getField("field_53450"),speed=livingState.getField("field_53451");
            Object source=Proxy.newProxyInstance(cl,new Class<?>[]{multiBuffer},(proxy,method,args)->{
                String name=method.getName();
                if(name.equals("getBuffer"))return sink;
                if(name.equals("toString"))return "InferedArmorGeometry";
                if(name.equals("hashCode"))return System.identityHashCode(proxy);
                if(name.equals("equals"))return proxy==args[0];
                if(method.isDefault())return InvocationHandler.invokeDefault(proxy,method,args);
                if(method.getReturnType()==void.class)return null;
                throw new UnsupportedOperationException(method.toString());
            });
            layers=layerField;armorLayer=layerClass;renderLayer=render;walkPos=pos;walkSpeed=speed;provider=source;
        } catch(ReflectiveOperationException|RuntimeException error) {
            layers=null;armorLayer=null;renderLayer=null;walkPos=null;walkSpeed=null;provider=null;
            armorBroken=true;
        }
        gameLoader=cl;
    }
    private static void addVertex(float x,float y,float z) {
        if(!Float.isFinite(x)||!Float.isFinite(y)||!Float.isFinite(z))throw new IllegalArgumentException("Nonfinite player vertex");
        if(floats+18>vertices.length)throw new IllegalStateException("Chams vertex budget exceeded");
        int q=quadVertices*3;quad[q]=x;quad[q+1]=y;quad[q+2]=z;
        if(++quadVertices==4){
            for(int index:TRIANGLES) { System.arraycopy(quad,index*3,vertices,floats,3); floats+=3; }
            quadVertices=0;
        }
    }
     




    public static synchronized float[] collect(Object client,Object[] players,float tick,double cx,double cy,double cz,boolean armor) throws Exception {
        init(client.getClass().getClassLoader());
        Object dispatcher=client.getClass().getMethod("method_1561").invoke(client);
        floats=0;quadVertices=0;
        for(int i=0;i<Math.min(players.length,MAX_PLAYERS);i++) {
            Object renderer=getRenderer.invoke(dispatcher,players[i]);
            Object state=updateState.invoke(renderer,players[i],tick);
            appendPose(renderer,state,cx,cy,cz,armor);
        }
        return Arrays.copyOf(vertices,floats);
    }
    private static void appendPose(Object renderer,Object state,double cx,double cy,double cz,boolean armor) throws Exception {
            Object stack=matrixStack.newInstance();
            Class<?> stateClass=Class.forName("net.minecraft.class_10017",true,gameLoader);
            Object positionOffset=offset.invoke(renderer,state);
            double ox=(double)vecX.invoke(positionOffset),oy=(double)vecY.invoke(positionOffset),oz=(double)vecZ.invoke(positionOffset);
            translate.invoke(stack,stateClass.getField("field_53325").getDouble(state)-cx+ox,stateClass.getField("field_53326").getDouble(state)-cy+oy,stateClass.getField("field_53327").getDouble(state)-cz+oz);
             
            Object sleeping=sleepDirection.get(state);
            if(sleeping!=null) {
                float eye=eyeHeight.getFloat(state)-0.1f;
                translate.invoke(stack,-((int)directionX.invoke(sleeping))* (double)eye,0.0,-((int)directionZ.invoke(sleeping))* (double)eye);
            }
            float s=baseScale.getFloat(state);
            scale.invoke(stack,s,s,s);
            transforms.invoke(renderer,state,stack,bodyYaw.getFloat(state),s);
            scale.invoke(stack,-1.0f,-1.0f,1.0f);
            rendererScale.invoke(renderer,state,stack);
            translate.invoke(stack,0.0,-1.501,0.0);
            Object model=getModel.invoke(renderer);
            setAngles.invoke(model,state);
            renderModel.invoke(model,stack,sink,0xF000F0,0,-1);
            if(quadVertices!=0)throw new IllegalStateException("Incomplete player model quad");
            if(armor) appendArmor(renderer,state,stack);
    }
     












    private static void appendArmor(Object renderer,Object state,Object stack) {
        if(armorBroken)return;
        if(layers==null||armorLayer==null||renderLayer==null||provider==null) {
            armorBroken=true;
            return;
        }
        int before=floats;
        try {
            Object list=layers.get(renderer);
            if(!(list instanceof Iterable))return;
            float pos=0.0f,speed=0.0f;
            if(walkPos!=null&&walkSpeed!=null) {
                try {
                    pos=((Number)walkPos.get(state)).floatValue();
                    speed=((Number)walkSpeed.get(state)).floatValue();
                } catch(RuntimeException ignored) {
                     
                }
            }
            for(Object layer:(Iterable<?>)list) {
                if(!armorLayer.isInstance(layer))continue;
                renderLayer.invoke(layer,stack,provider,0xF000F0,state,pos,speed);
            }
        } catch(ReflectiveOperationException|RuntimeException error) {
             
            armorBroken=true;
        }
         
         
        if(quadVertices!=0) { floats=before; quadVertices=0; armorBroken=true; }
    }
    public static synchronized float[] collectPoses(Object[] renderers,Object[] states,double cx,double cy,double cz) throws Exception {
        if(renderers.length!=states.length)throw new IllegalArgumentException("Pose count mismatch");
        if(renderers.length==0)return new float[0];
        init(renderers[0].getClass().getClassLoader());floats=0;quadVertices=0;
        for(int i=0;i<Math.min(renderers.length,MAX_PLAYERS);i++)appendPose(renderers[i],states[i],cx,cy,cz,false);
        return Arrays.copyOf(vertices,floats);
    }
     
    public static synchronized boolean armorUnavailable() { return armorBroken; }
     
    public static synchronized float[] collectModel(Object model,Object state,Object stack) throws Exception {
        init(model.getClass().getClassLoader());floats=0;quadVertices=0;
        setAngles.invoke(model,state);renderModel.invoke(model,stack,sink,0xF000F0,0,-1);
        if(quadVertices!=0)throw new IllegalStateException("Incomplete model quad");
        return Arrays.copyOf(vertices,floats);
    }
    public static synchronized void clear() { floats=0;quadVertices=0; }
}
