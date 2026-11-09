import java.util.Collections;
import java.util.Set;

 
public final class InferedXrayBridge {
    public static volatile boolean ACTIVE;
    public static volatile Set<Object> TARGET_STATES = Collections.emptySet();
    public static volatile Object INVISIBLE;

    public static boolean shouldHide(Object state) {
        return ACTIVE && !TARGET_STATES.contains(state);
    }

    public static Object decideRenderShape(Object state, Object vanillaShape) {
        Object invisible = INVISIBLE;
        if (!ACTIVE || invisible == null || TARGET_STATES.contains(state)) {
            return vanillaShape;
        }
        return invisible;
    }

     
    public static boolean decideOcclusion(boolean vanillaOcclusion) {
        return !ACTIVE && vanillaOcclusion;
    }

    private InferedXrayBridge() {}
}
