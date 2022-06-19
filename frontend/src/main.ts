import Vue from 'vue'
import axios from 'axios'
import VueAxios from 'vue-axios'
import VueCookies from 'vue-cookies'
import App from './App.vue'
import router from './router'
import vuetify from './plugins/vuetify'
import VueMarkdown from 'vue-markdown'

Vue.config.productionTip = false

Vue.use(VueAxios, axios)
Vue.use(VueCookies)
Vue.$cookies.config('7d', '', '', false, 'Strict')

new Vue({
    components: {
        VueMarkdown,
    },
    router,
    vuetify,
    render: h => h(App),
}).$mount('#app')
