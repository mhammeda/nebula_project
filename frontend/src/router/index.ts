import Vue from 'vue'
import Router from 'vue-router'
import LandingPage from '@/components/LandingPage.vue'
import Login from '@/components/util/Login.vue'
import Register from '@/components/util/Register.vue'
import HomePage from '@/components/HomePage.vue'
import AdminPage from '@/components/AdminPage.vue'
import ProfilePage from '@/components/ProfilePage.vue'
import changePasswordForm from '@/components/util/changePasswordForm.vue'
import displayRecoveryKey from '@/components/util/displayRecoveryKey.vue'
import FullPostPage from '@/components/posts/FullPostPage.vue'
import SearchPage from '@/components/searchPage.vue'
import CommunityPage from '@/components/communities/CommunityPage.vue'
import Page404 from '@/components/util/page404.vue'
import ChatDialogue from '@/components/chat/ChatDialogue.vue'

Vue.use(Router)

// Create our Router, heree we give it all the possible paths, as well as meta information
// such as requiresAuth.
const router = new Router({
    mode: 'history',
    routes: [
        {
            path: '/',
            name: 'landingPage',
            component: LandingPage,
        },
        {
            path: '/login',
            name: 'login',
            component: Login,
            meta: {
                guest: true,
            },
        },
        {
            path: '/register',
            name: 'register',
            component: Register,
            meta: {
                guest: true,
            },
        },
        {
            path: '/homepage',
            name: 'homepage',
            component: HomePage,
            meta: {
                requiresAuth: true,
            },
        },
        {
            path: '/adminPage',
            name: 'adminPage',
            component: AdminPage,
            meta: {
                requiresAuth: true,
            },
        },
        {
            path: '/changePasswordForm',
            name: 'changePasswordForm',
            component: changePasswordForm,
            meta: {},
        },
        {
            path: '/displayRecoveryKey',
            name: 'displayRecoveryKey',
            component: displayRecoveryKey,
            meta: {},
        },
        {
            path: '/post/:postid',
            name: 'FullPostPage',
            component: FullPostPage,
            meta: {
                requiresAuth: true,
            },
        },
        {
            path: '/search/:searchTerm',
            name: 'searchPage',
            component: SearchPage,
            meta: {
                requiresAuth: true,
            },
        },
        {
            path: '/chatDialogue',
            name: 'chatDialogue',
            component: ChatDialogue,
            meta: {
                requiresAuth: true,
            },
        },
        {
            path: '/user/:userID',
            name: 'profilePage',
            component: ProfilePage,
            meta: {
                requiresAuth: true,
            }
        },
        {
            path: '/community/:communityID',
            name: 'communityPage',
            component: CommunityPage,
            meta: {
                requiresAuth: true,
            },
        },
        // 404 page, needs to remain at bottom for pattern matching
        {
            path: '/*',
            name: 'page404',
            component: Page404,
        },
    ],
})

// This code checks the meta of the route being entered:
// - if the route requiresAuth, we check the jwt cookie from localStorage to see if they have a token
//      - else re re-direct to login
// - If route is for guest, check if user is logged in.
router.beforeEach((to, from, next) => {
    // Page requires Authorisation
    if (to.matched.some(record => record.meta.requiresAuth)) {
        // Check for token, this will of course need to be updated
        if (localStorage.getItem('loggedIn') == null) {
            console.log('got here')
            next({
                path: '/login',
                query: { nextUrl: to.path },
            })
        } else {
            // let user = JSON.parse(localStorage.getItem('user'))
            // can add further user checks here, such as isStudent, isTeacher or whatever.
            next({
                params: { nextUrl: to.path },
            })
        }

        // Else is a guest page
    } else if (to.matched.some(record => record.meta.guest)) {
        if (localStorage.getItem('loggedIn') == null) {
            next()
        } else {
            next({ name: 'homepage' })
        }

        // Otherwise we just redirect.
    } else {
        next()
    }
})

export default router
