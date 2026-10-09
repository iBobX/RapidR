// The manual's "Designing forms in RapidR Studio" (docs/manual/designing-forms.md):
// the tutorial "your first two-form app", a shot per step — the steps are
// Studio's own commands and test steps (ide/panels.inc PanelStep), the same
// ones tests/studio_add_form.mjs checks.

const NEW = "newproject:gui|{dir}|Multi,wait,wait";
const ADD_FORM = "project.addForm,wait,key:Enter,wait,wait,wait";
const FORM2 = "tool:RLABEL,prop:Caption=Hello from Form2,tool:REDIT,prop:Text=Type here,tool:RBUTTON,prop:Caption=Close";
const FORM2_EVENT = "event:OnClick,wait,type:Form2.Close,key:Escape";
const MAIN = "open:main.rr,wait,view.designer,wait,tool:RBUTTON,prop:Caption=Show Form2,event:OnClick,wait,type:Form2.Show,key:Escape";

export const scenes = [
  // 1. File > New Project > Form app: Form1 on its designer
  { topic: "designer", name: "01-new-project", studio: { do: NEW, delay: 6 }, crop: [0, 0, 1280, 770] },
  // 2. Project > Add Form: Form2.rr named in the tree (Enter), on its designer
  { topic: "designer", name: "02-add-form", studio: { do: `${NEW},${ADD_FORM}`, delay: 8 }, crop: [0, 0, 1280, 770] },
  // 3. The toolbox (RapidR's names)
  { topic: "designer", name: "03-toolbox", studio: { do: `${NEW},${ADD_FORM},focus:toolbox`, delay: 8 }, crop: [0, 570, 244, 200] },
  // 4. A label, an edit and a button on Form2, their captions set in the inspector
  { topic: "designer", name: "04-form2", studio: { do: `${NEW},${ADD_FORM},${FORM2},wait`, delay: 10 }, crop: [244, 64, 1036, 506] },
  // 5. The Events page: OnClick made the handler; Form2.Close typed in it
  { topic: "designer", name: "05-handler", studio: { do: `${NEW},${ADD_FORM},${FORM2},${FORM2_EVENT},wait`, delay: 10 }, crop: [244, 64, 790, 506] },
  // 6. main.rr: the $INCLUDE Add Form wrote, Form1's button showing Form2 (saved for 7)
  { topic: "designer", name: "06-main", studio: { do: `${NEW},${ADD_FORM},${FORM2},${FORM2_EVENT},${MAIN},file.saveAll,wait`, delay: 30 }, crop: [244, 64, 790, 506] },
  // 7. Run: Form1's button clicked, Form2 shown
  { topic: "designer", name: "07-run-form1", run: { from: "designer/06-main", dir: "Projects/Multi", file: "main.rr", events: "button2.onclick" }, window: 1 },
  { topic: "designer", name: "08-run-form2", run: { from: "designer/06-main", dir: "Projects/Multi", file: "main.rr", events: "button2.onclick" }, window: 2 },
];
