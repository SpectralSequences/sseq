import {LitElement, html, css} from 'lit-element';
import { styleMap } from 'lit-html/directives/style-map';

const RESIZER_WIDTH = 8;

export class Panel extends LitElement {
    static get properties() {
        return { 
            width : { type: Number },
            hidden : { type : Boolean },
            resizing : { type : Boolean }
        };
    }

    constructor(){
        super();
        this.resizing = false;
        this.hide = this.hide.bind(this);
        this.show = this.show.bind(this);
        this._startResize = this._startResize.bind(this);
        this._resize = this._resize.bind(this);
        this._stopResize = this._stopResize.bind(this);
        this.transitionTime = "0.5s";
        this.width = 240; // px
        this.minWidth = 200; // px
        this.collapsedWidth = 30; // px
        this.hidden = false;
        this.hidden_width = "2rem";
    }

    static get styles() {
        return css`
            [hidden] {
                display:none !important;
            }
            .divider {
                height : 100%;
                cursor : ew-resize;
                width : ${RESIZER_WIDTH}px;
                position : absolute;
                display:inline;
                z-index : 10000;
            }
            .sidebar {
                height: 100%;
                margin: 5px 5px 5px 0;
                margin-left : ${RESIZER_WIDTH / 2}px;
                border-left: 1px solid #DDD;
                float:left; display:inline;
            }

            .togglebtn {
                text-decoration: none;
                position:relative;
                font-size: 16pt;
                margin-left : -0.5px;
                cursor: pointer;
            }

            .togglebtn:hover {
                box-shadow: 0px 0px 5px #CCC;
            }

            .togglebtn:active {
                box-shadow: 0px 0px 8px #CCC;
                background-color: rgb(224, 224, 224);
                outline: none;
            }
            
        `
    }

    render(){
        let sidebar_styles  = { width : `${this.width}px` };
        if(!this.resizing){
            Object.assign(sidebar_styles, {transition : this.transitionTime});
        }
        if(this.hidden){
            let translation = this.width - this.collapsedWidth;
            Object.assign(sidebar_styles, {
                transform : `translateX(${translation}px)`,
                marginLeft : `-${translation}px`
            });
        }
        let content_styles = { width : "100%" };
        for(let key of ["display", "flexDirection", "flexWrap", "flexFlow", "justifyContent"]){
            if(this.style[key]){
                content_styles[key] = this.style[key];
            }
        }
        return html`
            <div class=divider @mousedown=${this._startResize} ?hidden="${this.hidden}"></div>
            <div class=sidebar style="${styleMap(sidebar_styles)}">
                <div style="display:flex; height:100%;">
                    <div>
                        <button @click=${this.hide} class="togglebtn" ?hidden="${this.hidden}">&times;</button>
                        <button @click=${this.show} class="togglebtn" ?hidden="${!this.hidden}">&#9776;</button>
                    </div>
                    <div class=content style="${styleMap(content_styles)}">
                        <div><button class="togglebtn" style="visibility:hidden;"></button></div>
                        <slot></slot>
                    </div>
                </div>
            </div>
        `;
    }

    _startResize(e){
        e.preventDefault();
        this.resizing = true;
        window.addEventListener('mousemove', this._resize);
        window.addEventListener('mouseup', this._stopResize);
        this.saved_body_cursor = document.body.style.cursor;
        document.body.style.cursor = "ew-resize";
    }

    _resize(e) {
        this.width = Math.max(this.getBoundingClientRect().right - e.pageX, this.minWidth);
    }

    _stopResize() {
        this.resizing = false;
        document.body.style.cursor = this.saved_body_cursor;
        window.removeEventListener('mousemove', this._resize);
        window.removeEventListener('mouseup', this._stopResize);
    }    

    hide(){
        this.hidden = true;
    }

    show(){
        this.hidden = false;
    }    
}
customElements.define('sseq-panel', Panel);