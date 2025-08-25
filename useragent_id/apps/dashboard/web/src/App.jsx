import React, { useState } from "react";
import { 
	Play, 
	Pause, 
	StopCircle, 
	RefreshCw, 
	Settings, 
	Plus, 
	Trash2, 
	Share2, 
	Edit, 
	Users, 
	Database, 
	Clock, 
	Code, 
	Brain, 
	Eye, 
	Terminal, 
	Folder, 
	FileText, 
	BarChart3, 
	Zap, 
	Cpu, 
	Lock, 
	Globe, 
	Save,
	Upload,
	Shield,
	Server,
	Activity,
	TrendingUp,
	PieChart,
	LineChart,
	Grid
} from "lucide-react";

const App = () => {
	const [activeTab, setActiveTab] = useState('dashboard');
	const [selectedAgent, setSelectedAgent] = useState(null);
	const [isRecording, setIsRecording] = useState(false);
	const [playbackSpeed, setPlaybackSpeed] = useState(1);
	const [selectedAutomation, setSelectedAutomation] = useState(null);
	const [showCreateModal, setShowCreateModal] = useState(false);
	const [newAgentName, setNewAgentName] = useState('');
	const [newAutomationName, setNewAutomationName] = useState('');
	const [systemStatus, setSystemStatus] = useState({
		wasm: 'active',
		memory: 'connected',
		vm: 'running',
		security: 'secure'
	});

	const [agents, setAgents] = useState([
		{ id: 1, name: 'Web Scraper Pro', type: 'Data Extraction', status: 'active', tasksCompleted: 156, lastActive: '2 hours ago', language: 'Python', confidence: 98, efficiency: 92 },
		{ id: 2, name: 'Form Filler Expert', type: 'Automation', status: 'active', tasksCompleted: 89, lastActive: '15 minutes ago', language: 'JavaScript', confidence: 95, efficiency: 88 },
		{ id: 3, name: 'API Integrator', type: 'Integration', status: 'paused', tasksCompleted: 42, lastActive: '1 day ago', language: 'Rust', confidence: 85, efficiency: 76 },
		{ id: 4, name: 'Content Analyzer', type: 'AI Processing', status: 'active', tasksCompleted: 203, lastActive: '30 minutes ago', language: 'Python', confidence: 99, efficiency: 94 }
	]);

	const [automations, setAutomations] = useState([
		{ id: 1, name: 'Daily Report Generator', agent: 'Web Scraper Pro', status: 'scheduled', schedule: 'Every day at 9:00 AM', lastRun: 'Today 9:00 AM', successRate: 98, executionTime: '2.3s' },
		{ id: 2, name: 'Lead Capture System', agent: 'Form Filler Expert', status: 'running', schedule: 'Real-time', lastRun: 'Just now', successRate: 95, executionTime: '1.8s' },
		{ id: 3, name: 'Social Media Poster', agent: 'API Integrator', status: 'paused', schedule: 'Every 2 hours', lastRun: 'Yesterday 5:00 PM', successRate: 92, executionTime: '3.1s' },
		{ id: 4, name: 'Sentiment Analysis', agent: 'Content Analyzer', status: 'scheduled', schedule: 'Every 6 hours', lastRun: 'Today 6:00 AM', successRate: 99, executionTime: '4.7s' }
	]);

	const [trainingData, setTrainingData] = useState([
		{ id: 1, name: 'E-commerce Checkout Flow', type: 'Session Replay', size: '245MB', date: '2023-12-15', quality: 'high', complexity: 'high' },
		{ id: 2, name: 'CRM Data Entry', type: 'DOM Mapping', size: '89MB', date: '2023-12-14', quality: 'medium', complexity: 'medium' },
		{ id: 3, name: 'API Authentication', type: 'Code Snippet', size: '12MB', date: '2023-12-13', quality: 'high', complexity: 'high' },
		{ id: 4, name: 'Dashboard Navigation', type: 'Session Replay', size: '187MB', date: '2023-12-12', quality: 'high', complexity: 'medium' }
	]);

	const [sessionReplays, setSessionReplays] = useState([
		{ id: 1, name: 'Checkout Process', agent: 'Web Scraper Pro', duration: '4:32', date: '2023-12-15', status: 'completed', success: true, confidence: 98 },
		{ id: 2, name: 'Form Submission', agent: 'Form Filler Expert', duration: '2:18', date: '2023-12-15', status: 'completed', success: true, confidence: 95 },
		{ id: 3, name: 'API Call Sequence', agent: 'API Integrator', duration: '6:45', date: '2023-12-14', status: 'failed', success: false, confidence: 67 },
		{ id: 4, name: 'Content Analysis', agent: 'Content Analyzer', duration: '8:23', date: '2023-12-14', status: 'completed', success: true, confidence: 99 }
	]);

	// Optional lead collection endpoint (configure via .env as VITE_LEADS_ENDPOINT)
	const LEADS_ENDPOINT = import.meta.env?.VITE_LEADS_ENDPOINT || '';
	const [lead, setLead] = useState({ name: '', email: '', company: '', note: '', website: '' }); // website = honeypot
	const [leadState, setLeadState] = useState({ sending: false, done: false, error: '' });

	async function submitLead(e) {
		e.preventDefault();
		setLeadState({ sending: true, done: false, error: '' });
		try {
			if (LEADS_ENDPOINT) {
				const res = await fetch(LEADS_ENDPOINT, {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({
						...lead,
						// basic context
						href: typeof window !== 'undefined' ? window.location.href : '',
						ua: typeof navigator !== 'undefined' ? navigator.userAgent : '',
						ts: new Date().toISOString()
					})
				});
				if (!res.ok) throw new Error(`HTTP ${res.status}`);
				setLeadState({ sending: false, done: true, error: '' });
				setLead({ name: '', email: '', company: '', note: '', website: '' });
				return;
			}
			// Fallback: mailto
			const subject = encodeURIComponent('UserAgent.ID – Demo request');
			const body = encodeURIComponent(
				`Name: ${lead.name}\nEmail: ${lead.email}\nCompany: ${lead.company}\nMessage: ${lead.note}\nPage: ${typeof window!== 'undefined' ? window.location.href : ''}`
			);
			window.location.href = `mailto:founder@useragent.id?subject=${subject}&body=${body}`;
			setLeadState({ sending: false, done: true, error: '' });
			setLead({ name: '', email: '', company: '', note: '', website: '' });
		} catch (err) {
			setLeadState({ sending: false, done: false, error: 'Could not submit. Try email instead.' });
		}
	}

	return (
		<div className="min-h-screen bg-gradient-to-b from-[#0b0f17] to-[#0e1422] text-gray-100">
			{/* Landing Hero */}
			<header className="relative overflow-hidden">
				<div className="mx-auto max-w-6xl px-6 pt-16 pb-10">
					<div className="flex flex-col lg:flex-row items-start lg:items-center gap-8">
						<div className="flex-1">
							<span className="inline-flex items-center gap-2 rounded-full border border-white/10 px-3 py-1 text-xs text-white/70">
								<Zap className="w-3.5 h-3.5 text-yellow-400" /> Human-in-the-loop automation
							</span>
							<h1 className="mt-4 text-4xl md:text-5xl font-extrabold tracking-tight">
								Teach small agents to do real work
							</h1>
							<p className="mt-4 max-w-2xl text-white/70">
								Record. Learn. Replay. UserAgent.ID captures your actions, learns the task, and trains tiny agents to automate it—safely and visibly.
							</p>
							<div className="mt-6 flex flex-wrap items-center gap-3">
								<a href="#demo" className="inline-flex items-center gap-2 rounded-md bg-blue-500 hover:bg-blue-600 px-4 py-2 font-medium">
									<Play className="w-4 h-4" /> Watch demo
								</a>
								<a href="#contact" className="inline-flex items-center gap-2 rounded-md border border-white/10 px-4 py-2 font-medium">
									<Share2 className="w-4 h-4" /> Get in touch
								</a>
							</div>
							<div className="mt-6 flex gap-6 text-white/50 text-xs">
								<div className="flex items-center gap-2"><Shield className="w-4 h-4" /> Privacy-first</div>
								<div className="flex items-center gap-2"><Cpu className="w-4 h-4" /> Local-first runtime</div>
								<div className="flex items-center gap-2"><Globe className="w-4 h-4" /> Web + Desktop</div>
							</div>
						</div>
						<div className="flex-1 w-full">
							<div className="relative rounded-xl border border-white/10 bg-white/5 backdrop-blur p-4">
								<div className="text-sm text-white/70">Replay with captions</div>
								<div className="mt-2 h-40 rounded-md bg-black/40 border border-white/10 flex items-center justify-center">
									<span className="text-white/50">Demo video placeholder</span>
								</div>
								<div className="mt-3 flex items-center gap-2 text-xs text-white/60">
									<Play className="w-3.5 h-3.5" /> Timestamp-aligned captions · Progress · Step/Stop
								</div>
							</div>
						</div>
					</div>
				</div>
			</header>

			{/* Features */}
			<section id="features" className="mx-auto max-w-6xl px-6 py-12">
				<h2 className="text-2xl font-bold">Why it’s different</h2>
				<div className="mt-6 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
					{[{icon: Brain, title: 'Learns from your actions', desc: 'DOM-aware steps with CSS-first selectors and smart Enter→Execute mapping.'},
					  {icon: Eye, title: 'Transparent replay', desc: 'Caption overlays aligned by timestamps; progress and step-through control.'},
					  {icon: Lock, title: 'Safe by design', desc: 'Local-first, privacy-first. Agents run where your data lives.'},
					  {icon: Terminal, title: 'Developer-friendly', desc: 'Rust core, Node/CDP replayer, and a web prototype with Tailwind.'},
					  {icon: BarChart3, title: 'Measurable tasks', desc: 'JSONL logs, learned steps, and repeatable runs for real work.'},
					  {icon: Grid, title: 'Modular pipeline', desc: 'Monitor → Learn → Train/Run, designed to plug into your stack.'}
					].map(({icon:Icon, title, desc}) => (
						<div key={title} className="rounded-lg border border-white/10 bg-white/5 p-4">
							<div className="flex items-center gap-2 text-white">
								<Icon className="w-4 h-4 text-blue-300" />
								<div className="font-medium">{title}</div>
							</div>
							<p className="mt-2 text-sm text-white/70">{desc}</p>
						</div>
					))}
				</div>
			</section>

			{/* CTA */}
			<section id="contact" className="mx-auto max-w-6xl px-6 pb-16">
				<div className="rounded-xl border border-white/10 bg-gradient-to-r from-blue-500/10 to-violet-500/10 p-6">
					<div className="grid grid-cols-1 md:grid-cols-3 gap-6 items-start">
						<div className="md:col-span-1">
							<h3 className="text-xl font-semibold">Want a private demo?</h3>
							<p className="text-white/70 text-sm mt-1">We’re onboarding early design partners. Share your workflow; we’ll show an agent do it.</p>
							<p className="text-white/50 text-xs mt-3">We keep it simple: no spam, no sharing. You’ll hear from us soon.</p>
						</div>
						<form onSubmit={submitLead} className="md:col-span-2 grid grid-cols-1 sm:grid-cols-2 gap-3">
							{/* Honeypot for bots */}
							<input type="text" name="website" autoComplete="off" value={lead.website} onChange={e=>setLead(v=>({...v, website: e.target.value}))} className="hidden" tabIndex="-1" />

							<input required type="text" name="name" placeholder="Your name" value={lead.name} onChange={e=>setLead(v=>({...v, name: e.target.value}))} className="rounded-md border border-white/10 bg-black/30 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/50" />
							<input required type="email" name="email" placeholder="Email" value={lead.email} onChange={e=>setLead(v=>({...v, email: e.target.value}))} className="rounded-md border border-white/10 bg-black/30 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/50" />
							<input type="text" name="company" placeholder="Company (optional)" value={lead.company} onChange={e=>setLead(v=>({...v, company: e.target.value}))} className="rounded-md border border-white/10 bg-black/30 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/50 sm:col-span-2" />
							<textarea name="note" placeholder="What would you like to automate?" value={lead.note} onChange={e=>setLead(v=>({...v, note: e.target.value}))} rows={3} className="rounded-md border border-white/10 bg-black/30 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/50 sm:col-span-2" />
							<div className="flex items-center gap-3 sm:col-span-2">
								<button disabled={leadState.sending} type="submit" className="inline-flex items-center gap-2 rounded-md bg-blue-500 hover:bg-blue-600 px-4 py-2 text-sm font-medium disabled:opacity-60">
									{leadState.sending ? 'Sending…' : (<><Share2 className="w-4 h-4" /> Request a demo</>)}
								</button>
								<a href="mailto:founder@useragent.id" className="text-sm text-white/70 hover:text-white">Or email us directly</a>
								{leadState.done && <span className="text-green-400 text-sm">Thanks — we’ll reach out shortly.</span>}
								{leadState.error && <span className="text-red-400 text-sm">{leadState.error}</span>}
							</div>
						</form>
					</div>
				</div>
			</section>

			{/* Existing prototype note */}
			<div className="px-6 pb-10">
				<div className="rounded-lg border border-white/10 bg-white/5 p-4">
					<h4 className="font-semibold">Dashboard prototype</h4>
					<p className="text-sm text-white/70">This is the web prototype. The native Rust dashboard has fuller functionality for recording and running agents locally.</p>
				</div>
			</div>
		</div>
	);
};

export default App;
