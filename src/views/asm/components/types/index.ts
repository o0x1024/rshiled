

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

