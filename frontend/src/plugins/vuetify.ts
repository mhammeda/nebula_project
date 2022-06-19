import Vue from 'vue'
import Vuetify from 'vuetify'
import 'vuetify/dist/vuetify.min.css'
import '@mdi/font/css/materialdesignicons.css'

import colors from 'vuetify/lib/util/colors'

Vue.use(Vuetify)

export default new Vuetify({
    icons: {
        iconfont: 'mdi',
    },
    theme: {
        themes: {
            light: {
                primary: 'white',
                secondary: '#1d1135',
                accent: '#ffff99',
                error: colors.red.accent3,
            },
            dark: {
                primary: '#314455',
                secondary: '#C96567',
                accent: '#97AABD',
            },
        },
    },
})
