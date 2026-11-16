use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    TopDown,
    LeftRight,
    BottomTop,
    RightLeft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramKind {
    Flowchart,
    Class,
    State,
    Sequence,
    Er,
    Pie,
    Mindmap,
    Journey,
    Timeline,
    Gantt,
    Requirement,
    GitGraph,
    C4,
    Sankey,
    Quadrant,
    ZenUML,
    Block,
    Packet,
    Kanban,
    Architecture,
    Radar,
    Treemap,
    XYChart,
    // vendored: diagram types mermaid documents that upstream had no
    // parser for. Each is a graph of nodes, edges and groups, which is why
    // these four and not the other four: see VENDORED.md.
    Swimlane,
    UseCase,
    TreeView,
    EventModeling,
    Wardley,
    Cynefin,
    Venn,
    Ishikawa,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceFrameKind {
    Alt,
    Opt,
    Loop,
    Par,
    Rect,
    Critical,
    Break,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceNotePosition {
    LeftOf,
    RightOf,
    Over,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateNotePosition {
    LeftOf,
    RightOf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceActivationKind {
    Activate,
    Deactivate,
}

#[derive(Debug, Clone)]
pub struct SequenceActivation {
    pub participant: String,
    pub index: usize,
    pub kind: SequenceActivationKind,
}

#[derive(Debug, Clone)]
pub struct SequenceNote {
    pub position: SequenceNotePosition,
    pub participants: Vec<String>,
    pub label: String,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct PieSlice {
    pub label: String,
    pub value: f32,
}

#[derive(Debug, Clone)]
pub struct QuadrantPoint {
    pub label: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GanttStatus {
    Done,
    Active,
    Crit,
    Milestone,
}

#[derive(Debug, Clone, Default)]
pub struct QuadrantData {
    pub title: Option<String>,
    pub x_axis_left: Option<String>,
    pub x_axis_right: Option<String>,
    pub y_axis_bottom: Option<String>,
    pub y_axis_top: Option<String>,
    pub quadrant_labels: [Option<String>; 4], // top-right, top-left, bottom-left, bottom-right
    pub points: Vec<QuadrantPoint>,
}

#[derive(Debug, Clone)]
pub struct GanttTask {
    pub id: String,
    pub label: String,
    pub start: Option<String>,
    pub duration: Option<String>,
    pub after: Option<String>,
    pub section: Option<String>,
    pub status: Option<GanttStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitGraphCommitType {
    Normal,
    Reverse,
    Highlight,
    Merge,
    CherryPick,
}

#[derive(Debug, Clone)]
pub struct GitGraphCommit {
    pub id: String,
    pub message: Option<String>,
    pub seq: usize,
    pub commit_type: GitGraphCommitType,
    pub custom_type: Option<GitGraphCommitType>,
    pub tags: Vec<String>,
    pub parents: Vec<String>,
    pub branch: String,
    pub custom_id: bool,
}

#[derive(Debug, Clone)]
pub struct GitGraphBranch {
    pub name: String,
    pub order: Option<f32>,
    pub insertion_index: usize,
}

#[derive(Debug, Clone, Default)]
pub struct GitGraphData {
    pub main_branch: String,
    pub commits: Vec<GitGraphCommit>,
    pub branches: Vec<GitGraphBranch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum C4ShapeKind {
    Person,
    ExternalPerson,
    System,
    SystemDb,
    SystemQueue,
    ExternalSystem,
    ExternalSystemDb,
    ExternalSystemQueue,
    Container,
    ContainerDb,
    ContainerQueue,
    ExternalContainer,
    ExternalContainerDb,
    ExternalContainerQueue,
    Component,
    ComponentDb,
    ComponentQueue,
    ExternalComponent,
    ExternalComponentDb,
    ExternalComponentQueue,
}

impl C4ShapeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            C4ShapeKind::Person => "person",
            C4ShapeKind::ExternalPerson => "external_person",
            C4ShapeKind::System => "system",
            C4ShapeKind::SystemDb => "system_db",
            C4ShapeKind::SystemQueue => "system_queue",
            C4ShapeKind::ExternalSystem => "external_system",
            C4ShapeKind::ExternalSystemDb => "external_system_db",
            C4ShapeKind::ExternalSystemQueue => "external_system_queue",
            C4ShapeKind::Container => "container",
            C4ShapeKind::ContainerDb => "container_db",
            C4ShapeKind::ContainerQueue => "container_queue",
            C4ShapeKind::ExternalContainer => "external_container",
            C4ShapeKind::ExternalContainerDb => "external_container_db",
            C4ShapeKind::ExternalContainerQueue => "external_container_queue",
            C4ShapeKind::Component => "component",
            C4ShapeKind::ComponentDb => "component_db",
            C4ShapeKind::ComponentQueue => "component_queue",
            C4ShapeKind::ExternalComponent => "external_component",
            C4ShapeKind::ExternalComponentDb => "external_component_db",
            C4ShapeKind::ExternalComponentQueue => "external_component_queue",
        }
    }
}

#[derive(Debug, Clone)]
pub struct C4Shape {
    pub id: String,
    pub label: String,
    pub type_label: Option<String>,
    pub techn: Option<String>,
    pub descr: Option<String>,
    pub sprite: Option<String>,
    pub tags: Option<String>,
    pub link: Option<String>,
    pub parent_boundary: String,
    pub kind: C4ShapeKind,
    pub bg_color: Option<String>,
    pub border_color: Option<String>,
    pub font_color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct C4Boundary {
    pub id: String,
    pub label: String,
    pub boundary_type: String,
    pub descr: Option<String>,
    pub sprite: Option<String>,
    pub tags: Option<String>,
    pub link: Option<String>,
    pub parent_boundary: String,
    pub bg_color: Option<String>,
    pub border_color: Option<String>,
    pub font_color: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum C4RelKind {
    Rel,
    BiRel,
    RelUp,
    RelDown,
    RelLeft,
    RelRight,
    RelBack,
}

#[derive(Debug, Clone)]
pub struct C4Rel {
    pub kind: C4RelKind,
    pub from: String,
    pub to: String,
    pub label: String,
    pub techn: Option<String>,
    pub descr: Option<String>,
    pub sprite: Option<String>,
    pub tags: Option<String>,
    pub link: Option<String>,
    pub offset_x: f32,
    pub offset_y: f32,
    pub line_color: Option<String>,
    pub text_color: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct C4Data {
    pub shapes: Vec<C4Shape>,
    pub boundaries: Vec<C4Boundary>,
    pub rels: Vec<C4Rel>,
    pub c4_type: Option<String>,
    pub c4_shape_in_row_override: Option<usize>,
    pub c4_boundary_in_row_override: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct SequenceBox {
    pub label: Option<String>,
    pub color: Option<String>,
    pub participants: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StateNote {
    pub position: StateNotePosition,
    pub target: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct SequenceFrameSection {
    pub label: Option<String>,
    pub start_idx: usize,
    pub end_idx: usize,
}

#[derive(Debug, Clone)]
pub struct SequenceFrame {
    pub kind: SequenceFrameKind,
    pub sections: Vec<SequenceFrameSection>,
    pub start_idx: usize,
    pub end_idx: usize,
}

impl Direction {
    pub fn from_token(token: &str) -> Option<Self> {
        let upper = token.to_ascii_uppercase();
        match upper.as_str() {
            "TD" | "TB" => Some(Self::TopDown),
            "BT" => Some(Self::BottomTop),
            "LR" => Some(Self::LeftRight),
            "RL" => Some(Self::RightLeft),
            _ => None,
        }
    }

    pub fn from_timeline_token(token: &str) -> Option<Self> {
        let upper = token.to_ascii_uppercase();
        match upper.as_str() {
            "TD" => Some(Self::TopDown),
            "LR" => Some(Self::LeftRight),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub shape: NodeShape,
    pub value: Option<f32>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NodeLink {
    pub url: String,
    pub title: Option<String>,
    pub target: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub start_label: Option<String>,
    pub end_label: Option<String>,
    pub directed: bool,
    pub arrow_start: bool,
    pub arrow_end: bool,
    pub arrow_start_kind: Option<EdgeArrowhead>,
    pub arrow_end_kind: Option<EdgeArrowhead>,
    pub start_decoration: Option<EdgeDecoration>,
    pub end_decoration: Option<EdgeDecoration>,
    pub style: EdgeStyle,
}

/// Port direction on an architecture-beta service (`db:L -- R:server`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArchDir {
    Left,
    Right,
    Top,
    Bottom,
}

impl ArchDir {
    pub fn from_char(ch: char) -> Option<Self> {
        match ch.to_ascii_uppercase() {
            'L' => Some(ArchDir::Left),
            'R' => Some(ArchDir::Right),
            'T' => Some(ArchDir::Top),
            'B' => Some(ArchDir::Bottom),
            _ => None,
        }
    }

    pub fn is_horizontal(self) -> bool {
        matches!(self, ArchDir::Left | ArchDir::Right)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeStyle {
    Solid,
    Dotted,
    Thick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDecoration {
    Circle,
    Cross,
    Diamond,
    DiamondFilled,
    // Crow's foot notation for ER diagrams
    CrowsFootOne,      // || exactly one
    CrowsFootZeroOne,  // o| zero or one
    CrowsFootMany,     // |{ one or many
    CrowsFootZeroMany, // o{ zero or many
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeArrowhead {
    OpenTriangle,
    ClassDependency,
}

#[derive(Debug, Clone)]
pub struct Subgraph {
    pub id: Option<String>,
    pub label: String,
    pub nodes: Vec<String>,
    pub direction: Option<Direction>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Graph {
    pub kind: DiagramKind,
    pub direction: Direction,
    pub nodes: BTreeMap<String, Node>,
    pub node_order: HashMap<String, usize>,
    pub edges: Vec<Edge>,
    pub subgraphs: Vec<Subgraph>,
    pub sequence_participants: Vec<String>,
    pub sequence_frames: Vec<SequenceFrame>,
    pub sequence_notes: Vec<SequenceNote>,
    pub sequence_activations: Vec<SequenceActivation>,
    pub sequence_autonumber: Option<usize>,
    pub sequence_boxes: Vec<SequenceBox>,
    pub state_notes: Vec<StateNote>,
    pub pie_slices: Vec<PieSlice>,
    pub pie_title: Option<String>,
    pub pie_show_data: bool,
    pub quadrant: QuadrantData,
    pub gantt_tasks: Vec<GanttTask>,
    pub gantt_title: Option<String>,
    pub gantt_sections: Vec<String>,
    pub gantt_display_mode: Option<String>,
    pub journey_title: Option<String>,
    // vendored: radar's `title` was parsed and dropped, and render_radar
    // emitted its title element with the text hardcoded empty.
    pub radar_title: Option<String>,
    // vendored: a Wardley map's components and their coordinates.
    pub wardley: WardleyData,
    // vendored: a Cynefin framework diagram's five domains.
    pub cynefin: CynefinData,
    // vendored: a Venn diagram's sets and the labels in their overlaps.
    pub venn: VennData,
    // vendored: an Ishikawa diagram's problem and its causes.
    pub ishikawa: IshikawaData,
    pub gitgraph: GitGraphData,
    pub class_defs: HashMap<String, NodeStyle>,
    pub node_classes: HashMap<String, Vec<String>>,
    pub node_styles: HashMap<String, NodeStyle>,
    pub subgraph_styles: HashMap<String, NodeStyle>,
    pub subgraph_classes: HashMap<String, Vec<String>>,
    pub node_links: HashMap<String, NodeLink>,
    pub edge_styles: HashMap<usize, EdgeStyleOverride>,
    pub edge_style_default: Option<EdgeStyleOverride>,
    pub c4: C4Data,
    pub mindmap: MindmapData,
    pub xychart: XYChartData,
    pub timeline: TimelineData,
    pub block: Option<BlockDiagram>,
    /// Architecture-beta edge port directions, keyed by edge index.
    /// `(from_port, to_port)` as written in `from:PORT -- PORT:to`.
    pub arch_edge_ports: HashMap<usize, (Option<ArchDir>, Option<ArchDir>)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeShape {
    Rectangle,
    ForkJoin,
    RoundRect,
    Stadium,
    Subroutine,
    Cylinder,
    ActorBox,
    Circle,
    DoubleCircle,
    Diamond,
    Hexagon,
    Parallelogram,
    ParallelogramAlt,
    Trapezoid,
    TrapezoidAlt,
    Asymmetric,
    MindmapDefault,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MindmapNodeType {
    Default,
    RoundedRect,
    Rect,
    Circle,
    Cloud,
    Bang,
    Hexagon,
}

#[derive(Debug, Clone)]
pub struct MindmapNode {
    pub id: String,
    pub label: String,
    pub level: usize,
    pub section: Option<usize>,
    pub node_type: MindmapNodeType,
    pub icon: Option<String>,
    pub class: Option<String>,
    pub children: Vec<String>,
}

/// vendored: an Ishikawa, or fishbone, diagram.
///
/// The problem is the head, the categories are the bones off the spine, and
/// indentation is the structure. A tree carries the same information, but the
/// spine and its angled ribs are what an author asking for a fishbone wants,
/// so this has a layout of its own.
#[derive(Debug, Clone, Default)]
pub struct IshikawaData {
    /// The first line: the event the causes are causes of.
    pub problem: Option<String>,
    pub categories: Vec<IshikawaCategory>,
}

#[derive(Debug, Clone)]
pub struct IshikawaCategory {
    pub name: String,
    pub causes: Vec<IshikawaCause>,
}

#[derive(Debug, Clone)]
pub struct IshikawaCause {
    pub text: String,
    /// 0 for a cause of the category, 1 and up for a cause of a cause.
    pub depth: usize,
}

/// vendored: a Venn diagram.
///
/// There is no node-and-edge reading of this one: a `union` is a label placed
/// in the region where the sets it names overlap, so the overlaps have to be
/// real geometry before the label has anywhere to go.
#[derive(Debug, Clone, Default)]
pub struct VennData {
    pub title: Option<String>,
    pub sets: Vec<VennSet>,
    pub unions: Vec<VennUnion>,
}

#[derive(Debug, Clone)]
pub struct VennSet {
    pub id: String,
    /// The display label from `set A["Alpha"]`, or the id.
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct VennUnion {
    /// The set ids this union names, in source order.
    pub sets: Vec<String>,
    pub label: String,
}

/// vendored: the five domains of a Cynefin diagram.
///
/// The names are fixed keywords and the positions never move: complex top
/// left, complicated top right, chaotic bottom left, clear bottom right,
/// confusion in the middle. An item's domain *is* what the diagram says about
/// it, which is why this has a layout of its own rather than being boxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CynefinDomain {
    Complex,
    Complicated,
    Chaotic,
    Clear,
    Confusion,
}

impl CynefinDomain {
    /// Every domain, in the order their items are stored.
    pub const ALL: [CynefinDomain; 5] = [
        CynefinDomain::Complex,
        CynefinDomain::Complicated,
        CynefinDomain::Chaotic,
        CynefinDomain::Clear,
        CynefinDomain::Confusion,
    ];

    pub fn from_keyword(word: &str) -> Option<Self> {
        match word.to_ascii_lowercase().as_str() {
            "complex" => Some(CynefinDomain::Complex),
            "complicated" => Some(CynefinDomain::Complicated),
            "chaotic" => Some(CynefinDomain::Chaotic),
            "clear" => Some(CynefinDomain::Clear),
            "confusion" | "disorder" => Some(CynefinDomain::Confusion),
            _ => None,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            CynefinDomain::Complex => "Complex",
            CynefinDomain::Complicated => "Complicated",
            CynefinDomain::Chaotic => "Chaotic",
            CynefinDomain::Clear => "Clear",
            CynefinDomain::Confusion => "Confusion",
        }
    }

    pub fn index(self) -> usize {
        CynefinDomain::ALL
            .iter()
            .position(|d| *d == self)
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct CynefinData {
    pub title: Option<String>,
    /// Items per domain, indexed by [`CynefinDomain::index`].
    pub items: [Vec<String>; 5],
    /// `complex --> complicated : "label"`.
    pub links: Vec<CynefinLink>,
}

#[derive(Debug, Clone)]
pub struct CynefinLink {
    pub from: CynefinDomain,
    pub to: CynefinDomain,
    pub label: Option<String>,
}

/// vendored: a Wardley map.
///
/// Mermaid writes a position as `[visibility, evolution]` -- visibility first,
/// which its own documentation flags as "opposite of typical (x, y) notation".
/// Both run 0 to 1: visibility from infrastructure to user-facing, evolution
/// from genesis to commodity.
#[derive(Debug, Clone, Default)]
pub struct WardleyData {
    pub title: Option<String>,
    /// The evolution axis's stages, left to right. Four by default.
    pub stages: Vec<String>,
    pub components: Vec<WardleyComponent>,
    /// `A -> B`, by component name.
    pub links: Vec<(String, String)>,
    /// `evolve <name> <evolution>`: where a component is headed.
    pub evolutions: Vec<(String, f32)>,
    /// `note "text" [visibility, evolution]`.
    pub notes: Vec<WardleyNote>,
}

#[derive(Debug, Clone)]
pub struct WardleyComponent {
    pub name: String,
    pub visibility: f32,
    pub evolution: f32,
    /// An `anchor` is the user need a map hangs from, drawn differently.
    pub anchor: bool,
}

#[derive(Debug, Clone)]
pub struct WardleyNote {
    pub text: String,
    pub visibility: f32,
    pub evolution: f32,
}

#[derive(Debug, Clone, Default)]
pub struct MindmapData {
    pub nodes: Vec<MindmapNode>,
    pub root_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XYSeriesKind {
    Bar,
    Line,
}

#[derive(Debug, Clone)]
pub struct XYSeries {
    pub kind: XYSeriesKind,
    pub label: Option<String>,
    pub values: Vec<f32>,
}

#[derive(Debug, Clone, Default)]
pub struct XYChartData {
    pub title: Option<String>,
    pub x_axis_label: Option<String>,
    pub x_axis_categories: Vec<String>,
    pub y_axis_label: Option<String>,
    pub y_axis_min: Option<f32>,
    pub y_axis_max: Option<f32>,
    pub series: Vec<XYSeries>,
}

#[derive(Debug, Clone)]
pub struct TimelineEvent {
    pub time: String,
    pub events: Vec<String>,
    pub section: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TimelineData {
    pub title: Option<String>,
    pub events: Vec<TimelineEvent>,
    pub sections: Vec<String>,
    pub direction: Option<Direction>,
}

#[derive(Debug, Clone, Default)]
pub struct BlockDiagram {
    pub columns: Option<usize>,
    pub nodes: Vec<BlockNode>,
}

#[derive(Debug, Clone)]
pub struct BlockNode {
    pub id: String,
    pub span: usize,
    pub is_space: bool,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            kind: DiagramKind::Flowchart,
            direction: Direction::TopDown,
            nodes: BTreeMap::new(),
            node_order: HashMap::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
            sequence_participants: Vec::new(),
            sequence_frames: Vec::new(),
            sequence_notes: Vec::new(),
            sequence_activations: Vec::new(),
            sequence_autonumber: None,
            sequence_boxes: Vec::new(),
            state_notes: Vec::new(),
            pie_slices: Vec::new(),
            pie_title: None,
            pie_show_data: false,
            quadrant: QuadrantData::default(),
            gantt_tasks: Vec::new(),
            gantt_title: None,
            gantt_sections: Vec::new(),
            gantt_display_mode: None,
            journey_title: None,
            radar_title: None,
            wardley: WardleyData::default(),
            cynefin: CynefinData::default(),
            venn: VennData::default(),
            ishikawa: IshikawaData::default(),
            gitgraph: GitGraphData::default(),
            class_defs: HashMap::new(),
            node_classes: HashMap::new(),
            node_styles: HashMap::new(),
            subgraph_styles: HashMap::new(),
            subgraph_classes: HashMap::new(),
            node_links: HashMap::new(),
            edge_styles: HashMap::new(),
            edge_style_default: None,
            c4: C4Data::default(),
            mindmap: MindmapData::default(),
            xychart: XYChartData::default(),
            timeline: TimelineData::default(),
            block: None,
            arch_edge_ports: HashMap::new(),
        }
    }

    pub fn ensure_node(&mut self, id: &str, label: Option<String>, shape: Option<NodeShape>) {
        let is_new = !self.nodes.contains_key(id);
        let entry = self.nodes.entry(id.to_string()).or_insert(Node {
            id: id.to_string(),
            label: id.to_string(),
            shape: NodeShape::Rectangle,
            value: None,
            icon: None,
        });
        if is_new {
            let order = self.node_order.len();
            self.node_order.insert(id.to_string(), order);
        }
        if let Some(label) = label {
            entry.label = label;
        }
        if let Some(shape) = shape {
            entry.shape = shape;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeStyle {
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub text_color: Option<String>,
    pub stroke_width: Option<f32>,
    pub stroke_dasharray: Option<String>,
    pub line_color: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EdgeStyleOverride {
    pub stroke: Option<String>,
    pub stroke_width: Option<f32>,
    pub dasharray: Option<String>,
    pub label_color: Option<String>,
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}
