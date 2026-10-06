# Real mouse/keyboard input for checking desktop GUI builds by hand (macOS):
# python3 tools/real_input.py PID c:x,y (click, window coords incl. title bar) d:x1,y1,x2,y2 (drag) k:keycode s:out.png
# Needs Accessibility + Screen Recording for the Claude Code helper app.
# The UI kernel's windows take it as a user's input; run the program without
# RAPIDR_CAPTURE / RAPIDR_TEST_EVENTS (under a test the kernel drops user input).
# PID is the program's own process (pgrep -x name), not a shell around it.
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
    elif kind=="d":
        # drag: d:x1,y1,x2,y2 (the left button held from one point to the other)
        x1,y1,x2,y2=[float(v) for v in arg.split(",")]; x1+=b['X']; x2+=b['X']; y1+=b['Y']; y2+=b['Y']
        mouse(Quartz.kCGEventMouseMoved,x1,y1); time.sleep(0.1); mouse(Quartz.kCGEventLeftMouseDown,x1,y1); time.sleep(0.1)
        for k in range(1,11):
            mouse(Quartz.kCGEventLeftMouseDragged,x1+(x2-x1)*k/10,y1+(y2-y1)*k/10); time.sleep(0.03)
        mouse(Quartz.kCGEventLeftMouseUp,x2,y2)
    elif kind=="k":
        # k:code or k:code+cmd+shift+alt+ctrl (modifiers held)
        code,*mods=arg.split("+")
        flags=0
        for m in mods: flags|={"cmd":Quartz.kCGEventFlagMaskCommand,"shift":Quartz.kCGEventFlagMaskShift,"alt":Quartz.kCGEventFlagMaskAlternate,"ctrl":Quartz.kCGEventFlagMaskControl}[m]
        for d in (True,False):
            e=Quartz.CGEventCreateKeyboardEvent(None,int(code),d)
            if flags: Quartz.CGEventSetFlags(e,flags)
            Quartz.CGEventPost(Quartz.kCGHIDEventTap,e); time.sleep(0.05)
    elif kind=="t":
        # t:text — each character typed (as the keyboard's Unicode string)
        for ch in arg:
            for d in (True,False):
                e=Quartz.CGEventCreateKeyboardEvent(None,{" ":49,"\n":36}.get(ch,0),d)
                if ch not in " \n": Quartz.CGEventKeyboardSetUnicodeString(e,len(ch),ch)
                Quartz.CGEventPost(Quartz.kCGHIDEventTap,e); time.sleep(0.03)
    elif kind=="s":
        subprocess.run(["screencapture","-x","-R%d,%d,%d,%d"%(b['X'],b['Y'],b['Width'],b['Height']),arg])
    time.sleep(0.3)
