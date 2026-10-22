# Real mouse/keyboard input for checking desktop GUI builds by hand (macOS):
# python3 tools/real_input.py PID c:x,y (click, window coords incl. title bar) C:x,y (click, screen coords) d:x1,y1,x2,y2 (drag) k:keycode s:out.png
#   m:x,y (the mouse moved there: hover), w:seconds (wait), r:x,y,n (the wheel, n notches down; negative: up),
#   k:cmd+shift+keycode (modifiers: cmd, shift, ctrl, alt), kd:code / ku:code (a key held down, let go),
#   t:text (typed)
# Needs Accessibility + Screen Recording for the Claude Code helper app.
# The UI kernel's windows take it as a user's input; run the program without
# RAPIDR_CAPTURE / RAPIDR_TEST_EVENTS (under a test the kernel drops user input).
# PID is the program's own process (pgrep -x name), not a shell around it.
# act.py PID action... : actions "c:x,y" (click), "k:code" (key), "s:file" (screenshot of the window)
import Quartz,time,sys,subprocess,os
pid=int(sys.argv[1])
def windows():
    return [w for w in Quartz.CGWindowListCopyWindowInfo(Quartz.kCGWindowListOptionOnScreenOnly,Quartz.kCGNullWindowID) if w.get('kCGWindowLayer')==0]
def front():
    # (activated only when another program is in front: activating it again
    # gives the key to its main window, away from a window it just opened)
    # RAPIDR_REAL_INPUT_STRICT=1: never activated — another program in front
    # (the person at the Mac clicked elsewhere) stops the input here, so no
    # click or key lands in their windows
    ws=windows()
    if (not ws or ws[0]['kCGWindowOwnerPID']!=pid) and os.environ.get("RAPIDR_REAL_INPUT_STRICT")=="1":
        sys.exit("not frontmost (strict): "+str(ws[0].get('kCGWindowOwnerName') if ws else None))
    if not ws or ws[0]['kCGWindowOwnerPID']!=pid:
        subprocess.run(["osascript","-e",f'tell application "System Events" to set frontmost of (first process whose unix id is {pid}) to true'])
        time.sleep(0.4)
        ws=windows()
    if not ws or ws[0]['kCGWindowOwnerPID']!=pid: sys.exit("not frontmost: "+str(ws[0].get('kCGWindowOwnerName') if ws else None))
    return dict([w for w in ws if w['kCGWindowOwnerPID']==pid][0]['kCGWindowBounds'])
def mouse(t,x,y,c=1):
    e=Quartz.CGEventCreateMouseEvent(None,t,(x,y),Quartz.kCGMouseButtonLeft); Quartz.CGEventSetIntegerValueField(e,Quartz.kCGMouseEventClickState,c); Quartz.CGEventSetFlags(e,0); Quartz.CGEventPost(Quartz.kCGHIDEventTap,e)
for a in sys.argv[2:]:
    b=front(); kind,arg=a.split(":",1)
    if kind in ("c","C"):
        # c: in the window; C: on the screen (points, as macOS accessibility
        # gives an element's frame: tools/macos/ui_probe.swift)
        x,y=[float(v) for v in arg.split(",")]
        if kind=="c": x+=b['X']; y+=b['Y']
        mouse(Quartz.kCGEventMouseMoved,x,y); time.sleep(0.1); mouse(Quartz.kCGEventLeftMouseDown,x,y); time.sleep(0.05); mouse(Quartz.kCGEventLeftMouseUp,x,y)
    elif kind=="d":
        # drag: d:x1,y1,x2,y2 (the left button held from one point to the other)
        x1,y1,x2,y2=[float(v) for v in arg.split(",")]; x1+=b['X']; x2+=b['X']; y1+=b['Y']; y2+=b['Y']
        mouse(Quartz.kCGEventMouseMoved,x1,y1); time.sleep(0.1); mouse(Quartz.kCGEventLeftMouseDown,x1,y1); time.sleep(0.1)
        for k in range(1,11):
            mouse(Quartz.kCGEventLeftMouseDragged,x1+(x2-x1)*k/10,y1+(y2-y1)*k/10); time.sleep(0.03)
        mouse(Quartz.kCGEventLeftMouseUp,x2,y2)
    elif kind=="m":
        x,y=[float(v) for v in arg.split(",")]; x+=b['X']; y+=b['Y']
        mouse(Quartz.kCGEventMouseMoved,x,y)
    elif kind=="w":
        time.sleep(float(arg))
    elif kind=="r":
        # r:x,y,n — the mouse wheel at (x, y) in the window, n notches (a
        # notch a line; positive scrolls the content down, as rolling the
        # wheel towards you), one notch every 16 ms
        x,y,n=[float(v) for v in arg.split(",")]; x+=b['X']; y+=b['Y']
        mouse(Quartz.kCGEventMouseMoved,x,y); time.sleep(0.1)
        for _ in range(int(abs(n))):
            e=Quartz.CGEventCreateScrollWheelEvent(None,Quartz.kCGScrollEventUnitLine,1,-1 if n>0 else 1)
            Quartz.CGEventSetFlags(e,0); Quartz.CGEventPost(Quartz.kCGHIDEventTap,e); time.sleep(0.016)
    elif kind=="k":
        # k:cmd+shift+code or k:code+cmd+shift (modifiers held; the number is the key code)
        parts=arg.split("+"); code=int(next(p for p in parts if p.isdigit())); flags=0
        for m in parts:
            if m.isdigit(): continue
            flags|={"cmd":Quartz.kCGEventFlagMaskCommand,"shift":Quartz.kCGEventFlagMaskShift,"ctrl":Quartz.kCGEventFlagMaskControl,"alt":Quartz.kCGEventFlagMaskAlternate}[m]
        for d in (True,False):
            e=Quartz.CGEventCreateKeyboardEvent(None,code,d)
            # (always set: an event with none takes the machine's held modifiers — a Cmd+Fn left down made a plain F5 Cmd+F5, which turned VoiceOver on)
            Quartz.CGEventSetFlags(e,flags)
            Quartz.CGEventPost(Quartz.kCGHIDEventTap,e); time.sleep(0.05)
    elif kind in ("kd","ku"):
        # kd:code / ku:code — a key pressed and held, then let go (a held
        # modifier: kd:59 is Control down, ku:59 Control up; between them
        # w: and s: — what shows while it is held)
        code=int(arg); down=kind=="kd"
        mods={59:Quartz.kCGEventFlagMaskControl,62:Quartz.kCGEventFlagMaskControl,55:Quartz.kCGEventFlagMaskCommand,56:Quartz.kCGEventFlagMaskShift,58:Quartz.kCGEventFlagMaskAlternate}
        e=Quartz.CGEventCreateKeyboardEvent(None,code,down)
        Quartz.CGEventSetFlags(e,mods.get(code,0) if down else 0)
        Quartz.CGEventPost(Quartz.kCGHIDEventTap,e)
    elif kind=="t":
        # text typed (t:RapidR Dark) by System Events, as the keyboard's
        # keys (an event carrying only a Unicode string is dropped by
        # macOS' input methods now and then)
        subprocess.run(["osascript","-e",'tell application "System Events" to keystroke "%s"'%arg.replace('\\','\\\\').replace('"','\\"')])
    elif kind=="s":
        subprocess.run(["screencapture","-x","-R%d,%d,%d,%d"%(b['X'],b['Y'],b['Width'],b['Height']),arg])
    time.sleep(0.3)
