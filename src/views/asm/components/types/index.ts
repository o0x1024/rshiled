

export interface Task {
	id: string
	name: string
	icp_no?: string
	rootdomain_count?: number
	domain_count?: number
	ips_count?: number
	port_count?: number
	website_count?: number
	api_count?: number
	webcomp_count?: number
	risk_count?: number
	monitor_status?: boolean
	running_status?: string
	next_run_time?: number
	last_run_time?: number
}

export interface Apis {
    id: number,
    task_id: number,
    method: string,
    uri: string,
	url: string,
	get_response: string,
	post_response: string,
    ufrom: string,
    status: number,
    get_body_length: number,
	post_body_length: number,
	update_at: number,
	count: number,
}

export interface RootDomain {
	id: number
	domain: string
	task_no: string
	task_name: number
	count: number
	create_at: number
	update_at: number
}

export interface ETPDomain {
	icp_name: string
	count: number
}

export interface Risk {
	id: number
	task_id: number
	risk_name: string
	risk_type: string
	risk_desc: string
	risk_level: string
	risk_detail: string
	risk_status: number
	ufrom: string
	update_at: number
}


export interface Domain {
	id: number
	domain: string
	aaa: string
	cname: string
	ns: string
	mx: string
	create_at: number
	update_at: number
}



export interface IPs {
	id: number
	task_id: string
	ip_addr: string
	create_at: number
	update_at: number
}



export interface WebSite {
	id: number
	task_id: string
	url: string
	favicon: string,   //图标的hash
	title: string,    //网站的标题
	headers: string,    //请求响应的头
	finger: Array<String>,    //网站指纹
	screenshot: string,     //网站的截图
	tags: Array<String>,
	ssl_info: string,      //网站证书信息
	create_at: number,    //创建时间
	update_at: number,
}


export interface WebComp {
	id: number,
	task_id: number,
	website: string,                 //组件URL
	comp_name: string,     //组件名称
	ctype: string,              //组件类型
	create_at: number,           //创建时间
	update_at: number,
}