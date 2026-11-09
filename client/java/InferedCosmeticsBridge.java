import java.awt.Graphics2D;
import java.awt.RenderingHints;
import java.awt.image.BufferedImage;
import java.io.ByteArrayInputStream;
import java.lang.reflect.*;
import javax.imageio.ImageIO;

 


public final class InferedCosmeticsBridge {
    public static volatile boolean CAPE, WINGS, FREECAM, SCAFFOLD, MENU_OPEN, FREELOOK;
     



    public static volatile boolean CAPE_MINECRAFT;
     




    public static volatile float WING_SIZE = 1f, WING_FLAP_SPEED = 1f, WING_FLAP_AMOUNT = 1f, WING_GLOW = 1f, WING_OPACITY = 1f;
    public static volatile int WING_MEMBRANE = 0x942b3e, WING_GLOW_COLOR = 0xffa65c;
    private static ClassLoader loader;
    private static Object client, capeId;
    private static int capeSerial;
    private static byte[] capeBytes;
    private static Method vertex, color, uv, overlay, light, normal, getBuffer, drawLayer;
    private static Method sendMovement;
    private static Field playerField, worldField;
    private static boolean renderFailed;
    private static final long epoch = System.nanoTime();
    private static Object capePlayer, capeWorld;
    private static long capeFrame;
    private static boolean capeWasEnabled;
    private static final int CAPE_COLS = 7, CAPE_ROWS = 11;
    private static final double CAPE_STEP = 1.0 / 120.0;
    private static double[][] cloth, clothPrevious;
    private static double[] clothOrigin, clothInputOrigin;
    private static float clothYaw, clothInputYaw;
    private static boolean clothCrouch;
    private static double clothAccumulator, clothTime;
    private static double[] minecraftPrevious;
    private static double minecraftTilt;
    private InferedCosmeticsBridge() {}
    private static Class<?> c(String name) throws Exception { return Class.forName("net.minecraft." + name, true, loader); }

    public static synchronized void configure(Object mc, byte[] png) throws Exception {
        loader = mc.getClass().getClassLoader(); client = mc;
        capeBytes = png.clone(); renderFailed = false;
        resetCapeSession();
        Class<?> consumer = c("class_4588"), layer = c("class_1921");
        vertex = consumer.getMethod("method_22918", Class.forName("org.joml.Matrix4f", true, loader), float.class, float.class, float.class);
        color = consumer.getMethod("method_39415", int.class);
        uv = consumer.getMethod("method_22913", float.class, float.class);
        overlay = consumer.getMethod("method_22922", int.class);
        light = consumer.getMethod("method_60803", int.class);
        normal = consumer.getMethod("method_22914", float.class, float.class, float.class);
        getBuffer = c("class_4597").getMethod("getBuffer", layer);
        drawLayer = c("class_4597$class_4598").getMethod("method_22994", layer);
        playerField = mc.getClass().getField("field_1724"); worldField = mc.getClass().getField("field_1687");
        sendMovement = c("class_746").getDeclaredMethod("method_3136"); sendMovement.setAccessible(true);
    }

     
    public static byte[] blackCape(byte[] source) throws Exception {
        BufferedImage logo = ImageIO.read(new ByteArrayInputStream(source));
        if (logo == null) throw new IllegalArgumentException("Cape artwork is not a PNG");
        BufferedImage result = new BufferedImage(128, 208, BufferedImage.TYPE_INT_ARGB);
        Graphics2D g = result.createGraphics();
        try {
            g.setColor(java.awt.Color.BLACK); g.fillRect(0, 0, 128, 208);
            g.setRenderingHint(RenderingHints.KEY_INTERPOLATION, RenderingHints.VALUE_INTERPOLATION_BICUBIC);
            double scale = Math.min(112.0 / logo.getWidth(), 184.0 / logo.getHeight());
            int w = Math.max(1, (int)Math.round(logo.getWidth()*scale));
            int h = Math.max(1, (int)Math.round(logo.getHeight()*scale));
            g.drawImage(logo, (128-w)/2, (208-h)/2, w, h, null);
        } finally { g.dispose(); }
        java.io.ByteArrayOutputStream out = new java.io.ByteArrayOutputStream();
        if (!ImageIO.write(result, "png", out)) throw new IllegalStateException("PNG writer unavailable");
        return out.toByteArray();
    }

    private static Object capeTexture() throws Exception {
        if (capeId != null) return capeId;
        Object image = c("class_1011").getMethod("method_49277", byte[].class).invoke(null, (Object)blackCape(capeBytes));
        Object texture;
        try { texture = c("class_1043").getConstructor(c("class_1011")).newInstance(image); }
        catch (Exception e) { ((AutoCloseable)image).close(); throw e; }
         
        Object id = c("class_2960").getMethod("method_60655", String.class, String.class).invoke(null, "darkclient", "custom_cape_" + (++capeSerial));
        Object manager = client.getClass().getMethod("method_1531").invoke(client);
        try { c("class_1060").getMethod("method_4616", c("class_2960"), c("class_1044")).invoke(manager, id, texture); }
        catch (Exception e) { ((AutoCloseable)texture).close(); throw e; }
        capeId = id;
        return id;
    }

     
    public static void render(Object camera, Object view) {
        if (client == null || renderFailed || (!CAPE && !WINGS)) return;
        try {
            Object player = playerField.get(client);
            if (player == null || !(boolean)c("class_1297").getMethod("method_5805").invoke(player)) return;
            if (!FREECAM && !(boolean)c("class_4184").getMethod("method_19333").invoke(camera)) return;
            if ((boolean)c("class_1297").getMethod("method_5767").invoke(player)) return;
            float tick = (float)c("class_4184").getMethod("method_55437").invoke(camera);
            Object cp = c("class_4184").getMethod("method_19326").invoke(camera);
            Object pp = c("class_1297").getMethod("method_30950", float.class).invoke(player, tick);
            double[] pos = new double[3], worldPos = new double[3], cameraPos = new double[3];
            String[] coordinates = {"method_10216", "method_10214", "method_10215"};
            for (int i=0;i<3;i++) {
                worldPos[i] = (double)c("class_243").getMethod(coordinates[i]).invoke(pp);
                cameraPos[i] = (double)c("class_243").getMethod(coordinates[i]).invoke(cp);
                pos[i] = worldPos[i] - cameraPos[i];
            }
            float oldYaw = c("class_1309").getField("field_6220").getFloat(player);
            float yaw = c("class_1309").getField("field_6283").getFloat(player);
            yaw = oldYaw + wrap(yaw-oldYaw)*tick;
            boolean crouch = (boolean)c("class_1297").getMethod("method_18276").invoke(player);
            double time = (System.nanoTime()-epoch)*1e-9;
            Object buffers = client.getClass().getMethod("method_22940").invoke(client);
            Object immediate = c("class_4599").getMethod("method_23000").invoke(buffers);
            if (CAPE) {
                Object layer = c("class_1921").getMethod("method_23576", c("class_2960")).invoke(null, capeTexture());
                Object sink = getBuffer.invoke(immediate, layer);
                Object world = worldField.get(client);
                long now = System.nanoTime();
                double dt = capeFrame == 0 ? 0 : (now - capeFrame) * 1e-9;
                if (!capeWasEnabled || capePlayer != player || capeWorld != world || dt > .25) resetCape(worldPos, yaw, crouch);
                capePlayer = player; capeWorld = world; capeFrame = now;
                boolean paused = (boolean)client.getClass().getMethod("method_1493").invoke(client);
                double[][] mesh = capeMesh(worldPos, yaw, crouch, dt, paused);
                for (int q=0; q<mesh.length; q+=4) {
                    int cell=q/4, row=cell/(CAPE_COLS-1), col=cell%(CAPE_COLS-1);
                    float u0=col/(float)(CAPE_COLS-1), u1=(col+1)/(float)(CAPE_COLS-1);
                    float v0=row/(float)(CAPE_ROWS-1), v1=(row+1)/(float)(CAPE_ROWS-1);
                    float[][] tex={{u0,v0},{u0,v1},{u1,v1},{u1,v0}};
                    float[][] panel=new float[4][3];
                    for(int i=0;i<4;i++) for(int axis=0;axis<3;axis++) panel[i][axis]=(float)(mesh[q+i][axis]-cameraPos[axis]);
                    float[] faceNormal=capeNormal(panel);
                    for(int i=0;i<4;i++) emitCape(sink,view,panel[i],0xffffffff,tex[i][0],tex[i][1],faceNormal,1);
                    for(int i=3;i>=0;i--) emitCape(sink,view,panel[i],0xff000000,tex[i][0],tex[i][1],faceNormal,-1);
                }
                drawLayer.invoke(immediate, layer);
            }
            if (WINGS) {
                Object layer = c("class_1921").getMethod("method_49042").invoke(null);
                Object sink = getBuffer.invoke(immediate, layer);
                float[][] mesh = wingMesh(time);
                for (int i=0;i<mesh.length;i++) {
                    float[] v = mesh[i];
                    int shade = wingColor(v, time);
                    emit(sink, view, transform(v,pos,yaw,crouch), shade,0,0,false);
                }
                drawLayer.invoke(immediate, layer);
            }
            capeWasEnabled = CAPE;
            if (!CAPE) resetCapeSession();
        } catch (Exception error) {
            renderFailed = true;
            System.err.println("[Dark cosmetics] render disabled until reload: " + cause(error));
        }
    }
    private static float wrap(float degrees) { return (degrees%360+540)%360-180; }
    private static float[] transform(float[] v,double[] pos,float yaw,boolean crouch) {
        double a = Math.toRadians(yaw), y=v[1], z=v[2];
        if(crouch) { y-=.28; z+=.12; }
         
        return new float[]{(float)(pos[0]+v[0]*Math.cos(a)+z*Math.sin(a)),(float)(pos[1]+y),(float)(pos[2]+v[0]*Math.sin(a)-z*Math.cos(a))};
    }
    private static void emit(Object sink,Object matrix,float[] p,int argb,float u,float v,boolean textured) throws Exception {
        vertex.invoke(sink,matrix,p[0],p[1],p[2]); color.invoke(sink,argb);
        if(textured) { uv.invoke(sink,u,v); overlay.invoke(sink,0x000A0000); light.invoke(sink,0x00F000F0); normal.invoke(sink,0f,0f,-1f); }
    }
    private static void resetCapeSession() {
        cloth=null; clothPrevious=null; clothOrigin=clothInputOrigin=null;
        clothAccumulator=clothTime=0; capeFrame=0; capePlayer=capeWorld=null; capeWasEnabled=false;
        minecraftPrevious=null; minecraftTilt=0;
    }
     
    public static void resetCape(double[] origin,float yaw,boolean crouch) {
        if(origin.length!=3 || !Double.isFinite(yaw)) throw new IllegalArgumentException("Invalid cape anchor");
        for(double coordinate:origin) if(!Double.isFinite(coordinate)) throw new IllegalArgumentException("Invalid cape position");
        cloth=new double[CAPE_ROWS*CAPE_COLS][3]; clothPrevious=new double[cloth.length][3];
        clothOrigin=origin.clone(); clothInputOrigin=origin.clone(); clothYaw=clothInputYaw=yaw; clothCrouch=crouch; clothAccumulator=clothTime=0;
        minecraftPrevious=origin.clone();minecraftTilt=0;
        for(int row=0;row<CAPE_ROWS;row++) for(int col=0;col<CAPE_COLS;col++) {
            int i=row*CAPE_COLS+col;
            cloth[i]=new double[]{.31-.62*col/(CAPE_COLS-1),1.42-1.26*row/(CAPE_ROWS-1)-(crouch?.28:0),.19+(crouch?.12:0)};
            clothPrevious[i]=cloth[i].clone();
        }
    }
     





    public static double[][] capeMesh(double[] origin,float yaw,boolean crouch,double dt,boolean paused) {
        if(origin.length!=3 || !Double.isFinite(yaw) || !Double.isFinite(dt) || dt<0) throw new IllegalArgumentException("Invalid cape step");
        for(double coordinate:origin) if(!Double.isFinite(coordinate)) throw new IllegalArgumentException("Invalid cape position");
        if(CAPE_MINECRAFT) return minecraftCapeMesh(origin,yaw,crouch,dt,paused);
        double distance=clothOrigin==null?0:Math.sqrt(Math.pow(origin[0]-clothOrigin[0],2)+Math.pow(origin[1]-clothOrigin[1],2)+Math.pow(origin[2]-clothOrigin[2],2));
        if(cloth==null || distance>8 || dt>.25) resetCape(origin,yaw,crouch);
        if(!paused) {
            double speed=dt>1e-6?Math.min(8,Math.hypot(clothInputOrigin[0]-origin[0],clothInputOrigin[2]-origin[2])/dt):0;
            double carry=clothAccumulator, frameDt=Math.min(dt,.1);
            clothAccumulator+=frameDt;
            int steps=0;
            while(clothAccumulator+1e-9>=CAPE_STEP && steps++<12) {
                 
                 
                double f=frameDt>1e-9?Math.min(1,(steps*CAPE_STEP-carry)/frameDt):1;
                double[] stepOrigin=new double[3];
                for(int axis=0;axis<3;axis++)stepOrigin[axis]=clothInputOrigin[axis]+(origin[axis]-clothInputOrigin[axis])*f;
                float anchorYaw=clothInputYaw+wrap(yaw-clothInputYaw)*(float)f;
                rebaseCape(stepOrigin,anchorYaw,crouch);
                clothAccumulator=Math.max(0,clothAccumulator-CAPE_STEP); clothTime+=CAPE_STEP;
                for(int i=CAPE_COLS;i<cloth.length;i++) {
                    double[] p=cloth[i],previous=clothPrevious[i];
                    double falloff=(i/CAPE_COLS)/(double)(CAPE_ROWS-1);
                    double wind=(1.2+speed*4)*falloff;
                    double[] force={.6*falloff*Math.sin(clothTime*4+i*.35),-9.81,wind};
                    for(int axis=0;axis<3;axis++) {
                        double old=p[axis];
                        p[axis]+=(p[axis]-previous[axis])*.985+force[axis]*CAPE_STEP*CAPE_STEP;
                        previous[axis]=old;
                    }
                }
                for(int iteration=0;iteration<10;iteration++) {
                    pinCape();
                    for(int row=0;row<CAPE_ROWS;row++) for(int col=0;col<CAPE_COLS;col++) {
                        int i=row*CAPE_COLS+col;
                        if(col+1<CAPE_COLS) constrainCape(i,i+1,.62/(CAPE_COLS-1));
                        if(row+1<CAPE_ROWS) constrainCape(i,i+CAPE_COLS,1.26/(CAPE_ROWS-1));
                        if(row+1<CAPE_ROWS && col+1<CAPE_COLS) {
                            double diagonal=Math.hypot(.62/(CAPE_COLS-1),1.26/(CAPE_ROWS-1));
                            constrainCape(i,i+CAPE_COLS+1,diagonal); constrainCape(i+1,i+CAPE_COLS,diagonal);
                        }
                    }
                    for(int i=CAPE_COLS;i<cloth.length;i++) {
                         
                         
                        double[] p=cloth[i],anchor=cloth[i%CAPE_COLS];
                        double tx=p[0]-anchor[0],ty=p[1]-anchor[1],tz=p[2]-anchor[2];
                        double span=Math.sqrt(tx*tx+ty*ty+tz*tz);
                        double maxSpan=1.26*(i/CAPE_COLS)/(CAPE_ROWS-1)*1.02;
                        if(span>maxSpan) {
                            double scale=maxSpan/span;
                            p[0]=anchor[0]+tx*scale;p[1]=anchor[1]+ty*scale;p[2]=anchor[2]+tz*scale;
                        }
                         
                        double minimum=.17+(clothCrouch?.12:0);
                        if(cloth[i][2]<minimum) {cloth[i][2]=minimum;clothPrevious[i][2]=minimum;}
                         
                        if(p[1]>anchor[1]) {p[1]=anchor[1];clothPrevious[i][1]=p[1];}
                    }
                }
                pinCape();
            }
            clothInputOrigin=origin.clone();clothInputYaw=yaw;
        }
        double angle=Math.toRadians(clothYaw);
        double[][] out=new double[(CAPE_ROWS-1)*(CAPE_COLS-1)*4][3]; int n=0;
        for(int row=0;row<CAPE_ROWS-1;row++) for(int col=0;col<CAPE_COLS-1;col++) {
            int i=row*CAPE_COLS+col;
            for(int index:new int[]{i,i+CAPE_COLS,i+CAPE_COLS+1,i+1}) {
                double[] p=cloth[index];
                out[n++]=new double[]{clothOrigin[0]+p[0]*Math.cos(angle)+p[2]*Math.sin(angle),clothOrigin[1]+p[1],clothOrigin[2]+p[0]*Math.sin(angle)-p[2]*Math.cos(angle)};
            }
        }
        return out;
    }
     



    public static double[][] minecraftCapeMesh(double[] origin,float yaw,boolean crouch,double dt,boolean paused) {
        double tilt=minecraftTilt(origin,yaw,dt,paused);
        double angle=Math.toRadians(yaw),yawSin=Math.sin(angle),yawCos=Math.cos(angle);
        double shoulder=1.42-(crouch?.28:0),baseZ=.19+(crouch?.12:0);
        double tiltSin=Math.sin(tilt),tiltCos=Math.cos(tilt);
        double[][] out=new double[(CAPE_ROWS-1)*(CAPE_COLS-1)*4][3];int n=0;
        for(int row=0;row<CAPE_ROWS-1;row++) for(int col=0;col<CAPE_COLS-1;col++) {
            int i=row*CAPE_COLS+col;
            for(int index:new int[]{i,i+CAPE_COLS,i+CAPE_COLS+1,i+1}) {
                int r=index/CAPE_COLS,c=index%CAPE_COLS;
                double x=.31-.62*c/(CAPE_COLS-1),down=1.26*r/(CAPE_ROWS-1);
                double px=x,py=shoulder-down*tiltCos,pz=baseZ+down*tiltSin;
                out[n++]=new double[]{origin[0]+px*yawCos+pz*yawSin,origin[1]+py,origin[2]+px*yawSin-pz*yawCos};
            }
        }
        return out;
    }
     


    private static double minecraftTilt(double[] origin,float yaw,double dt,boolean paused) {
        if(minecraftPrevious==null
            || Math.hypot(origin[0]-minecraftPrevious[0],origin[2]-minecraftPrevious[2])>8) {
            minecraftPrevious=origin.clone();minecraftTilt=0;return 0;
        }
        if(dt>1e-6) {
            double dx=origin[0]-minecraftPrevious[0],dz=origin[2]-minecraftPrevious[2];
            double forward=(dx*Math.sin(Math.toRadians(yaw))+dz*Math.cos(Math.toRadians(yaw)))/dt;
            double target=Math.max(0,Math.min(1,forward/5.6))*Math.toRadians(55);
            if(!paused) minecraftTilt+=(target-minecraftTilt)*(1-Math.exp(-dt*12));
        }
        minecraftPrevious=origin.clone();
        return minecraftTilt;
    }
    private static void rebaseCape(double[] origin,float yaw,boolean crouch) {
        double oldAngle=Math.toRadians(clothYaw),angle=Math.toRadians(yaw);
        double dx=clothOrigin[0]-origin[0],dy=clothOrigin[1]-origin[1],dz=clothOrigin[2]-origin[2];
        for(double[][] grid:new double[][][]{cloth,clothPrevious}) for(double[] p:grid) {
            double wx=p[0]*Math.cos(oldAngle)+p[2]*Math.sin(oldAngle)+dx;
            double wz=p[0]*Math.sin(oldAngle)-p[2]*Math.cos(oldAngle)+dz;
            p[0]=wx*Math.cos(angle)+wz*Math.sin(angle);p[1]+=dy;p[2]=wx*Math.sin(angle)-wz*Math.cos(angle);
        }
        clothOrigin=origin;clothYaw=yaw;clothCrouch=crouch;pinCape();
    }
    private static void pinCape() {
        for(int col=0;col<CAPE_COLS;col++) {
            double[] p=cloth[col]; p[0]=.31-.62*col/(CAPE_COLS-1);p[1]=1.42-(clothCrouch?.28:0);p[2]=.19+(clothCrouch?.12:0);
            System.arraycopy(p,0,clothPrevious[col],0,3);
        }
    }
    private static void constrainCape(int a,int b,double length) {
        double[] p=cloth[a],q=cloth[b];
        double dx=q[0]-p[0],dy=q[1]-p[1],dz=q[2]-p[2];double distance=Math.sqrt(dx*dx+dy*dy+dz*dz);
        if(distance<1e-9)return;
        double factor=(distance-length)/distance;
        boolean pinA=a<CAPE_COLS,pinB=b<CAPE_COLS;
        double wa=pinA?0:pinB?1:.5,wb=pinB?0:pinA?1:.5;
        p[0]+=dx*factor*wa;p[1]+=dy*factor*wa;p[2]+=dz*factor*wa;
        q[0]-=dx*factor*wb;q[1]-=dy*factor*wb;q[2]-=dz*factor*wb;
    }
    private static float[] capeNormal(float[][] quad) {
        float[] a=quad[0],b=quad[1],d=quad[3];
        float ux=b[0]-a[0],uy=b[1]-a[1],uz=b[2]-a[2],vx=d[0]-a[0],vy=d[1]-a[1],vz=d[2]-a[2];
        float x=uy*vz-uz*vy,y=uz*vx-ux*vz,z=ux*vy-uy*vx;float length=(float)Math.sqrt(x*x+y*y+z*z);
        return length<1e-6?new float[]{0,0,-1}:new float[]{x/length,y/length,z/length};
    }
    private static void emitCape(Object sink,Object matrix,float[] p,int argb,float u,float v,float[] n,int side) throws Exception {
        vertex.invoke(sink,matrix,p[0],p[1],p[2]);color.invoke(sink,argb);uv.invoke(sink,u,v);
        overlay.invoke(sink,0x000A0000);light.invoke(sink,0x00F000F0);normal.invoke(sink,n[0]*side,n[1]*side,n[2]*side);
    }

     



    private static double wingPhase(double time) {
        double phase=(time*2.4*wingSpeed())%(2*Math.PI);
        return phase<0?phase+2*Math.PI:phase;
    }
    private static float wingFloat(float value,float fallback,float min,float max) {
        return Float.isFinite(value)?Math.max(min,Math.min(max,value)):fallback;
    }
    private static float wingSize() { return wingFloat(WING_SIZE,1f,.4f,1.6f); }
    private static float wingSpeed() { return wingFloat(WING_FLAP_SPEED,1f,0f,3f); }
    private static float wingAmount() { return wingFloat(WING_FLAP_AMOUNT,1f,0f,2f); }
    private static float wingGlow() { return wingFloat(WING_GLOW,1f,0f,2f); }
    private static float wingOpacity() { return wingFloat(WING_OPACITY,1f,0f,1f); }

     




    public static float[][] wingMesh(double time) {
        if(!Double.isFinite(time)) throw new IllegalArgumentException("Invalid wing time");
        double phase=wingPhase(time),emberPhase=phase/(2*Math.PI);
        float[][] out=new float[2*48*4][4];int n=0;
        float[] shoulder={.18f,1.40f,.24f},elbow={.46f,1.82f,.27f},wrist={.84f,1.94f,.30f};
        float[][] tips={{1.85f,1.75f,.34f},{1.74f,1.10f,.35f},{1.35f,.64f,.33f},{.79f,.39f,.29f},{.24f,.73f,.25f}};
        for(int side:new int[]{-1,1}) {
             
             
            for(int panel=0;panel<4;panel++) for(int segment=0;segment<3;segment++) for(int band=0;band<2;band++) {
                float u0=segment/3f,u1=(segment+1)/3f,v0=band/2f,v1=(band+1)/2f;
                float[][] q={wingSurface(wrist,tips[panel],tips[panel+1],u0,v0),wingSurface(wrist,tips[panel],tips[panel+1],u0,v1),
                    wingSurface(wrist,tips[panel],tips[panel+1],u1,v1),wingSurface(wrist,tips[panel],tips[panel+1],u1,v0)};
                n=appendWing(out,n,q,side,phase,0);
            }
             
            float[][] spar={shoulder,elbow,wrist,tips[0]};
            for(int i=0;i<3;i++) n=appendWing(out,n,wingRibbon(spar[i],spar[i+1],.045f-i*.01f,.035f-i*.01f,.015f),side,phase,1);
            for(int finger=1;finger<=4;finger++) {
                float[] tip=tips[finger];
                for(int segment=0;segment<3;segment++) {
                    float t0=segment/3f,t1=(segment+1)/3f;
                    float[] a=wingFinger(wrist,tip,t0),b=wingFinger(wrist,tip,t1);
                    n=appendWing(out,n,wingRibbon(a,b,.026f-.020f*t0,.026f-.020f*t1,.015f),side,phase,1);
                }
                 
                n=appendWing(out,n,wingRibbon(wingFinger(wrist,tip,.12f),wingFinger(wrist,tip,.84f),.007f,.003f,.032f),side,phase,2);
            }
            float[][] wristClaw={{.78f,1.92f,.34f},{.86f,2.08f,.35f},{.94f,1.99f,.35f},{.78f,1.92f,.34f}};
            float[][] tipClaw={{1.76f,1.76f,.36f},{1.94f,1.80f,.36f},{1.84f,1.69f,.36f},{1.76f,1.76f,.36f}};
            n=appendWing(out,n,wristClaw,side,phase,1);n=appendWing(out,n,tipClaw,side,phase,1);
             
            for(int ember=0;ember<3;ember++) {
                float[] tip=tips[ember];
                double drift=(emberPhase+ember/3.0)%1;
                float ex=tip[0]+.035f+(float)(drift*.08),ey=tip[1]+(float)(drift*.16);
                float r=(float)(.018*(1-drift)+.003);
                float[][] q={{ex,ey+r,.41f},{ex+r,ey,.41f},{ex,ey-r,.41f},{ex-r,ey,.41f}};
                n=appendWing(out,n,q,side,phase,3);
            }
        }
        return out;
    }
    private static float[] wingSurface(float[] wrist,float[] a,float[] b,float u,float v) {
        float[] edge=new float[3],p=new float[3];
        double scallop=.22*Math.sin(Math.PI*u);
        for(int axis=0;axis<3;axis++) {
            edge[axis]=a[axis]+(b[axis]-a[axis])*u;
            edge[axis]+=(wrist[axis]-edge[axis])*scallop;
            p[axis]=wrist[axis]+(edge[axis]-wrist[axis])*v;
        }
        p[0]+=.025f*(float)Math.sin(Math.PI*v);
        p[2]+=.10f*(float)(Math.sin(Math.PI*u)*Math.sin(Math.PI*v));
        return p;
    }
    private static float[] wingFinger(float[] wrist,float[] tip,float t) {
        float[] p=new float[3];
        for(int axis=0;axis<3;axis++) p[axis]=wrist[axis]+(tip[axis]-wrist[axis])*t;
        p[0]+=.025f*(float)Math.sin(Math.PI*t);
        return p;
    }
    private static float[][] wingRibbon(float[] a,float[] b,float startWidth,float endWidth,float raise) {
        double length=Math.hypot(b[0]-a[0],b[1]-a[1]);
        float nx=(float)(-(b[1]-a[1])/length),ny=(float)((b[0]-a[0])/length);
        float az=a[2]+(a[0]>.24f?raise:0),bz=b[2]+(b[0]>.24f?raise:0);
        return new float[][]{{a[0]+nx*startWidth,a[1]+ny*startWidth,az},{b[0]+nx*endWidth,b[1]+ny*endWidth,bz},
            {b[0]-nx*endWidth,b[1]-ny*endWidth,bz},{a[0]-nx*startWidth,a[1]-ny*startWidth,az}};
    }
    private static int appendWing(float[][] out,int n,float[][] quad,int side,double phase,int material) {
        double amount=wingAmount(),scale=.76*wingSize();
        for(float[] p:quad) {
             
            double reach=Math.max(0,(p[0]-.24)/1.70);
            double flap=.32+.20*amount*Math.sin(phase)+.035*amount*Math.sin(2*phase-.5)+.075*reach*amount*Math.sin(phase-.65);
            double x=(p[0]-.18)*scale,z=(p[2]-.24)*scale;
            out[n++]=new float[]{(float)(side*(.18+x*Math.cos(flap)-z*Math.sin(flap))),
                (float)(1.40+(p[1]-1.40)*scale+.018*reach*amount*Math.sin(phase-.65)),
                (float)Math.max(.24,.24+x*Math.sin(flap)+z*Math.cos(flap)),material};
        }
        return n;
    }
    private static int wingColor(float[] v,double time) {
        int material=(int)v[3];
        double height=Math.max(0,Math.min(1,(v[1]-.55)/1.4));
        if(material==0)return wingTint(wingShade(WING_MEMBRANE,.22),wingShade(WING_MEMBRANE,1.30),.25+.65*height);
        if(material==1)return wingTint(wingShade(WING_MEMBRANE,.18),wingShade(WING_MEMBRANE,.95),height);
        double pulse=.5+.5*Math.sin(wingPhase(time)-.6);
        if(material==2)return wingGlowTint(.30+.45*pulse);
        return wingGlowTint(pulse);
    }
     


    private static int wingShade(int rgb,double factor) {
        int out=0xff000000;
        for(int shift=16;shift>=0;shift-=8) out|=Math.min(255,(int)Math.round(((rgb>>shift)&255)*factor))<<shift;
        return out;
    }
     
    private static int wingGlowTint(double blend) {
        int tint=wingTint(wingShade(WING_GLOW_COLOR,.55),wingShade(WING_GLOW_COLOR,1.45),blend);
        double factor=wingGlow();
        int out=tint&0xff000000;
        for(int shift=16;shift>=0;shift-=8) out|=Math.min(255,(int)Math.round(((tint>>shift)&255)*factor))<<shift;
        return out;
    }
    private static int wingTint(int dark,int bright,double blend) {
        int rgb=((int)Math.round(255*wingOpacity()))<<24;
        for(int shift=16;shift>=0;shift-=8) {
            int a=(dark>>shift)&255,b=(bright>>shift)&255;
            rgb|=((int)(a+(b-a)*blend))<<shift;
        }
        return rgb;
    }

     



    public static void runVanillaMovement(Object player) {
        if (client == null) throw new IllegalStateException("Cosmetics bridge not configured");
        try { sendMovement.invoke(player); }
        catch(Exception error) { throw new IllegalStateException("Vanilla movement failed",cause(error)); }
    }
    private static Throwable cause(Exception e) { return e instanceof InvocationTargetException i ? i.getCause() : e; }

     
    public static int[][] placementTargets(double x,double y,double z,double vx,double vz) {
        int bx=(int)Math.floor(x), by=(int)Math.floor(y+.001)-1, bz=(int)Math.floor(z);
        int sx=Math.abs(vx)>.001?(vx>0?1:-1):0, sz=Math.abs(vz)>.001?(vz>0?1:-1):0;
        return new int[][]{{bx,by,bz},{(int)Math.floor(x+vx*2),by,(int)Math.floor(z+vz*2)}, {bx+sx,by,bz+sz},{bx+sx,by,bz},{bx,by,bz+sz}};
    }
     


    public static synchronized void setStyle(boolean minecraft) {
        CAPE_MINECRAFT=minecraft;
        resetCapeSession();
    }
     
    public static synchronized void setArtwork(byte[] png) throws Exception {
        blackCape(png);
        capeBytes = png.clone(); renderFailed = false;
        releaseCapeTexture();
    }
     
    private static void releaseCapeTexture() {
        if(capeId!=null && client!=null) {
            Object oldId=capeId;
            ((java.util.concurrent.Executor)client).execute(() -> {
                try {
                    Object manager=client.getClass().getMethod("method_1531").invoke(client);
                    c("class_1060").getMethod("method_4615",c("class_2960")).invoke(manager,oldId);
                } catch(Exception error) { System.err.println("[Dark cosmetics] texture cleanup failed: "+cause(error)); }
            });
        }
        capeId=null;
    }
    public static synchronized void clear() throws Exception {
        CAPE=WINGS=FREECAM=SCAFFOLD=MENU_OPEN=FREELOOK=false;CAPE_MINECRAFT=false;
        releaseCapeTexture();
        capeBytes=null; renderFailed=false;
        resetCapeSession();
         
    }
}
