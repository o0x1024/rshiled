

export interface Enterprise {
	id: string
	name: string
	icp_no?: string
	monitor_status?: boolean
	running_status?:string
}


export interface RootDomain {
	id: number
	domain: string
	enterprise_no: string
	enterprise_name: number
	count: number
	create_at: number
	update_at: number
}

export interface ETPDomain {
	icp_name: string
	count: number
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
	enterprise_id: string
	ip_addr: string
	create_at: number
	update_at: number
}



export interface WebSite {
	id: number
	enterprise_id: string
	url: string
     favicon:string,   //图标的hash
     title:string,    //网站的标题
     headers:string,    //请求响应的头
     finger:Array<String>,    //网站指纹
     screenshot:string,     //网站的截图
     tags:Array<String>,
     ssl_info:string,      //网站证书信息
     create_at: number,    //创建时间
     update_at: number,  
}


