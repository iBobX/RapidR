# Real mouse/keyboard input for checking desktop GUI builds by hand (macOS):
# python3 tools/real_input.py PID c:x,y (click, window coords incl. title bar) k:keycode s:out.png
# Needs Accessibility + Screen Recording for the Claude Code helper app.
# act.py PID action... : actions "c:x,y" (click), "k:code" (key), "s:file" (screenshot of the window)
import Quartz,time,sys,subprocess
pid=int(sys.argv[1])
def front():
    subprocess.run(["osascript","-e",f'tell application "System Events" to set frontmost of (first process whose unix id is {pid}) to true'])
    time.sleep(0.4)
    ws=[w for w in Quartz.CGWindowListCopyWindowInfo(Quartz.kCGWindowListOptionOnScreenOnly,Quartz.kCGNullWindowID) if w.get('kCGWindowLayer')==0]
    if not ws or ws[0]['kCGWindowOwnerPID']!=pid: sys.exit("not frontmost: "+str(ws[0].get('kCGWindowOwnerName') if ws else None))
    return dict([w for w in ws if w['kCGWindowOwnerPID']==pid][0]['kCGWindowBounds'])
def mouse(t,x,y,c=1):
    e=Quartz.CGEventCreateMouseEvent(None,t,(x,y),Quartz.kCGMouseButtonLeft); Quartz.CGEventSetIntegerValueField(e,Quartz.kCGMouseEventClickState,c); Quartz.CGEventPost(Quartz.kCGHIDEventTap,e)
for a in sys.argv[2:]:
    b=front(); kind,arg=a.split(":",1)
    if kind=="c":
        x,y=[float(v) for v in arg.split(",")]; x+=b['X']; y+=b['Y']
        mouse(Quartz.kCGEventMouseMoved,x,y); time.sleep(0.1); mouse(Quartz.kCGEventLeftMouseDown,x,y); time.sleep(0.05); mouse(Quartz.kCGEventLeftMouseUp,x,y)
    elif kind=="k":
        for d in (True,False): Quartz.CGEventPost(Quartz.kCGHIDEventTap,Quartz.CGEventCreateKeyboardEvent(None,int(arg),d)); time.sleep(0.05)
    elif kind=="s":
        subprocess.run(["screencapture","-x","-R%d,%d,%d,%d"%(b['X'],b['Y'],b['Width'],b['Height']),arg])
    time.sleep(0.3)
