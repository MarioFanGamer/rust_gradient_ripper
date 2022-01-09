; Enables three HDMA channels, used for fixed colour HDMA.
; Input:
;   r_chnl: Channel number for the red channel.
;   r_src:  Address or label to the red table.
;   g_chnl: Channel number for the green channel
;   g_src:  Address or label to the green table.
;   b_chnl: Channel number for the blue channel
;   b_src:  Address or label to the blue table.
; Note that the SNES only supports channels from 0 to 7.
; For SMW specifically, only channel 3 to 6 (7 with SA-1 Pack) are normally free.
macro hdma_three_channels(r_chnl, r_src, g_chnl, g_src, b_chnl, b_src)
	REP #$20
	LDA #$3200				; Mode 0 (one register, write once), fixed colour
	STA $4300+(<r_chnl><<4)
	STA $4300+(<g_chnl><<4)
	STA $4300+(<b_chnl><<4)
	LDA.w #<r_src>
	STA $4302+(<r_chnl><<4)
	LDA.w #<g_src>
	STA $4302+(<g_chnl><<4)
	LDA.w #<b_src>
	STA $4302+(<b_chnl><<4)
	SEP #$20
	LDA.b #<r_src>>>16
	STA $4304+(<r_chnl><<4)
	LDA.b #<g_src>>>16
	STA $4304+(<g_chnl><<4)
	LDA.b #<b_src>>>16
	STA $4304+(<b_chnl><<4)
	LDA (1<<<r_chnl>)|(1<<<g_chnl>)|(1<<<b_chnl>)
	TSB $0D9F|!addr
endmacro

; Enables two HDMA channels, used for fixed colour HDMA.
; Input:
;   s_chnl: Channel number for the single colour channel.
;   s_src:  Address or label to the single colour table.
;   d_chnl: Channel number for the dual colour channel
;   d_src:  Address or label to the dual colour table.
; Note that the SNES only supports channels from 0 to 7.
; For SMW specifically, only channel 3 to 6 (7 with SA-1 Pack) are normally free.
macro hdma_twoo_channels(s_chnl, s_src, d_chnl, d_src)
	REP #$20
	LDA #$3200				; Mode 0 (one register, write once), fixed colour
	STA $4300+(<s_chnl><<4)
	LDA #$3202				; Mode 2 (one register, write twice), fixed colour
	STA $4300+(<d_chnl><<4)
	LDA.w #<s_src>
	STA $4302+(<s_chnl><<4)
	LDA.w #<d_src>
	STA $4302+(<d_chnl><<4)
	SEP #$20
	LDA.b #<s_src>>>16
	STA $4304+(<s_chnl><<4)
	LDA.b #<d_src>>>16
	STA $4304+(<d_chnl><<4)
	LDA.b (1<<<s_chnl>)|(1<<<d_chnl>)
	TSB $0D9F|!addr
endmacro


; Enables one HDMA channels, used for CG-RAM HDMA.
; Input:
;   channel:       
;   table_source:  Address or label to the table.
; Note that the SNES only supports channels from 0 to 7.
; For SMW specifically, only channel 3 to 6 (7 with SA-1 Pack) are normally free.
macro hdma_cgram_indexed(channel, table_source)
	REP #$20
	LDA #$2103				; Mode 3 (two registers, write twice), fixed colour and CG-RAM
	STA $4300+(<channel><<4)
	LDA.w #<table_source>
	STA $4302+(<channel><<4)
	SEP #$20
	LDA.b #<table_source>>>16
	STA $4304+(<channel><<4)
	LDA.b 1<<<channel>
	TSB $0D9F|!addr
endmacro
